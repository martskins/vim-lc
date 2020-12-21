pub mod code_lens;
pub mod extensions;
pub mod text_document;
pub mod window;
pub mod workspace;

use std::path::Path;

use crate::language_client::{Context, LanguageClient};
use crate::rpc;
use crate::rpc::RPCClient;
use anyhow::Result;
use lsp_types::{
    notification::{self, Notification},
    request::{self, Request},
    ClientCapabilities, ClientInfo, HoverClientCapabilities, InitializeParams, InitializeResult,
    InitializedParams, TextDocumentClientCapabilities, TraceOption, Url,
};

impl<C, S> LanguageClient<C, S>
where
    C: RPCClient,
    S: RPCClient,
{
    // handles messages sent from vim to the language client
    pub async fn handle_message(&self, message: rpc::Message) -> Result<()> {
        let ctx = Context::new(&message, self);
        match message {
            rpc::Message::MethodCall(msg) => match msg.method.as_str() {
                "workspace/applyEdit" => {
                    let params: lsp_types::ApplyWorkspaceEditParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::lsp::workspace::apply_edit(&ctx, &params)?;
                }
                _ => log::debug!("unhandled server method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method.as_str() {
                "window/logMessage" => {
                    let params: lsp_types::LogMessageParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::vim::log_message(&ctx, params)?;
                }
                "textDocument/publishDiagnostics" => {
                    let params: lsp_types::PublishDiagnosticsParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::lsp::text_document::publish_diagnostics(&ctx, params)?;
                }
                "$/progress" => {
                    let params: lsp_types::ProgressParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::lsp::window::progress(&ctx, params)?;
                }
                "window/showMessage" => {
                    let params: lsp_types::ShowMessageParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::lsp::window::show_message(&ctx, params)?;
                }
                _ => log::debug!("unhandled server notification {}", msg.method),
            },
            rpc::Message::Output(_) => unreachable!(),
        }

        Ok(())
    }
}

#[allow(deprecated)]
pub async fn initialize<C, S>(ctx: &Context<C, S>) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    let server_command = ctx.server()?;
    let root_path = get_root_path(&Path::new(&ctx.filename), &ctx.language_id)?
        .to_string_lossy()
        .to_string();
    ctx.state
        .write()
        .roots
        .insert(ctx.language_id.clone(), root_path.clone() + "/");

    let message = InitializeParams {
        process_id: Some(ctx.state.read().process_id),
        root_path: Some(root_path),
        root_uri: Some(Url::from_directory_path(std::env::current_dir()?).unwrap()),
        initialization_options: server_command.initialization_options.clone(),
        capabilities: ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities {
                hover: Some(HoverClientCapabilities {
                    content_format: Some(ctx.config.hover.preferred_markup_kind.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        trace: Some(TraceOption::Verbose),
        workspace_folders: None,
        client_info: Some(ClientInfo {
            name: "vim-lc".into(),
            version: Some("1.0".into()),
        }),
        locale: None,
    };

    let res: InitializeResult = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::Initialize::METHOD, message)?;

    ctx.state
        .write()
        .server_capabilities
        .insert(ctx.language_id.clone(), res.capabilities);

    Ok(())
}

pub fn get_root_path<'a>(path: &'a Path, language_id: &str) -> Result<&'a Path> {
    match language_id {
        "rust" => traverse_up(path, dir_has_one(&["Cargo.toml"])),
        "php" => traverse_up(path, dir_has_one(&["composer.json"])),
        "javascript" | "typescript" | "javascript.jsx" | "typescript.tsx" => {
            traverse_up(path, dir_has_one(&["package.json"]))
        }
        "python" => traverse_up(
            path,
            dir_has_one(&["setup.py", "Pipfile", "requirements.txt", "pyproject.toml"]),
        ),
        "c" | "cpp" => traverse_up(path, dir_has_one(&["compile_commands.json"])),
        "cs" => traverse_up(path, is_dotnet_root),
        "java" => traverse_up(
            path,
            dir_has_one(&[
                "pom.xml",
                "settings.gradle",
                "settings.gradle.kts",
                "WORKSPACE",
            ]),
        ),
        "scala" => traverse_up(path, dir_has_one(&["build.sbt"])),
        "haskell" => traverse_up(path, dir_has_one(&["stack.yaml"])).or_else(|_| {
            traverse_up(path, |dir| {
                dir_contains_file(dir, |f| has_extension(f, "cabal"))
            })
        }),
        "go" => traverse_up(path, dir_has_one(&["go.mod"])),
        _ => Err(anyhow::anyhow!("Unknown languageId: {}", language_id)),
    }
    .or_else(|_| {
        traverse_up(path, |dir| {
            dir.join(".git").exists() || dir.join(".hg").exists() || dir.join(".svn").exists()
        })
    })
    .or_else(|_| {
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get parent dir! path: {:?}", path));
        log::warn!(
            "Unknown project type. Fallback to use dir as project root: {:?}",
            parent
        );
        parent
    })
}

fn is_dotnet_root(dir: &Path) -> bool {
    if dir.join("project.json").exists() {
        return true;
    }
    if !dir.is_dir() {
        return false;
    }

    let entries = match dir.read_dir() {
        Ok(entries) => entries,
        Err(_) => return false,
    };
    for entry in entries {
        if let Ok(entry) = entry {
            if entry.path().ends_with(".csproj") {
                return true;
            }
        }
    }

    false
}

fn has_extension(path: &Path, ext: &str) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()), Some(path_ext) if path_ext == ext)
}

fn dir_has_one<'a>(files: &'a [&str]) -> impl Fn(&'a Path) -> bool {
    move |dir| files.iter().any(|file| dir.join(file).exists())
}

fn traverse_up<'a, F>(path: &'a Path, predicate: F) -> Result<&'a Path>
where
    F: Fn(&'a Path) -> bool,
{
    if predicate(path) {
        return Ok(path);
    }

    let next_path = path.parent().ok_or_else(|| anyhow::anyhow!("Hit root"))?;

    traverse_up(next_path, predicate)
}

fn dir_contains_file<F>(path: &Path, predicate: F) -> bool
where
    F: Fn(&Path) -> bool,
{
    if let Ok(diriter) = path.read_dir() {
        for entry in diriter {
            if let Ok(entry) = entry {
                if predicate(&entry.path()) {
                    return true;
                }
            }
        }
    }

    false
}

pub fn shutdown<C, S>(ctx: &Context<C, S>) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    ctx.server
        .as_ref()
        .unwrap()
        .call(request::Shutdown::METHOD, ())?;
    Ok(())
}

pub fn exit<C, S>(ctx: &Context<C, S>) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    ctx.server
        .as_ref()
        .unwrap()
        .notify(notification::Exit::METHOD, ())?;
    Ok(())
}

pub fn initialized<C, S>(ctx: &Context<C, S>) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    ctx.server
        .as_ref()
        .unwrap()
        .notify(notification::Initialized::METHOD, InitializedParams {})?;
    Ok(())
}
