use super::VLC;
use crate::config::*;
use crate::rpc::RPCClient;
use crate::vim::*;
use crate::CONFIG;
use crate::LANGUAGE_CLIENT;
use crate::VIM;
use failure::Fallible;
use futures::executor::block_on;

impl<T> VLC<T>
where
    T: RPCClient,
{
    pub fn apply_edits(&self, edits: lsp_types::WorkspaceEdit) -> Fallible<()> {
        // TODO: This is terrible, fix it some day.
        let changes: lsp_types::DocumentChanges = edits.document_changes.unwrap();
        let changes: Vec<DocumentChanges> = match changes {
            lsp_types::DocumentChanges::Edits(edits) => edits
                .into_iter()
                .map(|tde| {
                    let tde: lsp_types::TextDocumentEdit = tde;
                    let text_document = tde
                        .text_document
                        .uri
                        .to_string()
                        .replace(self.root_path.as_str(), "");
                    DocumentChanges {
                        text_document: text_document.clone(),
                        changes: tde
                            .edits
                            .into_iter()
                            .map(|e| {
                                let mut lines: Vec<String> =
                                    e.new_text.split('\n').map(|s| s.to_owned()).collect();
                                let line_count = lines.len();
                                let mut first_line = block_on(
                                    LANGUAGE_CLIENT
                                        .get_line(&text_document, e.range.start.line + 1),
                                )
                                .unwrap();
                                first_line.replace_range(
                                    e.range.start.character as usize..first_line.len(),
                                    &lines[0],
                                );
                                lines[0] = first_line;

                                let mut last_line = block_on(
                                    LANGUAGE_CLIENT.get_line(&text_document, e.range.end.line + 1),
                                )
                                .unwrap();
                                last_line.replace_range(
                                    0..e.range.end.character as usize,
                                    &lines[line_count - 1],
                                );
                                lines[line_count - 1] = last_line;
                                BufChanges {
                                    start: Position {
                                        line: e.range.start.line,
                                        column: e.range.start.character,
                                    },
                                    end: Position {
                                        line: e.range.end.line,
                                        column: e.range.end.character,
                                    },
                                    lines,
                                }
                            })
                            .collect(),
                    }
                })
                .collect(),
            lsp_types::DocumentChanges::Operations(_) => vec![],
        };
        self.client.notify("applyEdits", changes)?;
        Ok(())
    }

    pub fn show_diagnostics(&self, mut diagnostics: Vec<Diagnostic>) -> Fallible<()> {
        diagnostics.iter_mut().for_each(|d| {
            d.text_document = d.text_document.replace(self.root_path.as_str(), "");
        });

        let quickfix_list: Vec<QuickfixItem> =
            diagnostics.clone().into_iter().map(|l| l.into()).collect();
        self.set_quickfix(quickfix_list)?;

        if CONFIG.diagnostics.show_signs {
            let signs: Vec<Sign> = diagnostics.into_iter().map(|l| l.into()).collect();
            self.set_signs(signs)?;
        }

        Ok(())
    }

    pub fn show_hover(&self, input: lsp_types::Hover) -> Fallible<()> {
        let filetype = match input.contents {
            lsp_types::HoverContents::Scalar(ref c) => match &c {
                lsp_types::MarkedString::String(_) => String::new(),
                lsp_types::MarkedString::LanguageString(s) => s.language.clone(),
            },
            lsp_types::HoverContents::Array(ref c) => {
                if c.is_empty() {
                    String::new()
                } else {
                    match c[0].clone() {
                        lsp_types::MarkedString::String(_) => String::new(),
                        lsp_types::MarkedString::LanguageString(s) => s.language,
                    }
                }
            }
            lsp_types::HoverContents::Markup(ref c) => match &c.kind {
                lsp_types::MarkupKind::Markdown => "markdown".into(),
                lsp_types::MarkupKind::PlainText => String::new(),
            },
        };

        let lines = match input.contents {
            lsp_types::HoverContents::Scalar(ref c) => match c.clone() {
                lsp_types::MarkedString::String(s) => s.split('\n').map(|s| s.to_owned()).collect(),
                lsp_types::MarkedString::LanguageString(s) => {
                    s.value.split('\n').map(|s| s.to_owned()).collect()
                }
            },
            lsp_types::HoverContents::Array(ref c) => {
                if c.is_empty() {
                    vec![]
                } else {
                    match c[0].clone() {
                        lsp_types::MarkedString::String(s) => {
                            s.split('\n').map(|s| s.to_owned()).collect()
                        }
                        lsp_types::MarkedString::LanguageString(s) => {
                            s.value.split('\n').map(|s| s.to_owned()).collect()
                        }
                    }
                }
            }
            lsp_types::HoverContents::Markup(c) => {
                c.value.split('\n').map(|s| s.to_owned()).collect()
            }
        };

        match CONFIG.hover.display_mode {
            DisplayMode::Preview => {
                let client = VIM.client.clone();
                client.notify("showPreview", PreviewContent { filetype, lines })?;
            }
            DisplayMode::FloatingWindow => {
                let client = VIM.client.clone();
                client.notify("showFloatingWindow", PreviewContent { filetype, lines })?;
            }
        }
        Ok(())
    }

    pub fn show_in_fzf<I: FZFItem>(&self, items: Vec<I>) -> Fallible<()> {
        let text: Vec<String> = items.into_iter().map(|i| i.text()).collect();
        let sink = I::sink();
        self.client
            .notify("showFZF", serde_json::json!({"items": text, "sink": sink}))?;

        Ok(())
    }

    pub fn show_locations(&self, input: Vec<Location>) -> Fallible<()> {
        if input.is_empty() {
            return Ok(());
        }

        if input.len() == 1 {
            return self.jump_to_location(input.first().cloned().unwrap());
        }

        let locations: Vec<LocationWithPreview> = input
            .into_iter()
            .map(|l| {
                let filename = l.filename.replace(self.root_path.as_str(), "");
                let text = block_on(LANGUAGE_CLIENT.get_line(&filename, l.position.line))
                    .unwrap_or_default();
                LocationWithPreview {
                    preview: text,
                    location: Location {
                        filename,
                        position: l.position,
                    },
                }
            })
            .collect();

        self.show_in_fzf(locations)?;
        Ok(())
    }

    pub fn jump_to_location(&self, input: Location) -> Fallible<()> {
        self.execute(vec![
            ExecuteParams {
                action: "execute".into(),
                command: format!(
                    "execute 'edit' '{}'",
                    input.filename.replace(self.root_path.as_str(), "")
                ),
            },
            ExecuteParams {
                action: "call".into(),
                command: format!("cursor({}, {})", input.position.line, input.position.column),
            },
        ])?;
        Ok(())
    }

    /// evaluates an expression in vim and waits for the response.
    pub fn eval<R: serde::de::DeserializeOwned>(&self, cmd: EvalParams) -> Fallible<R> {
        let client = VIM.client.clone();
        let res: R = client.call("eval", cmd)?;
        Ok(res)
    }

    /// evaluates multiple commands and returns a vec of values.
    pub fn execute(&self, cmd: Vec<ExecuteParams>) -> Fallible<Vec<serde_json::Value>> {
        let client = VIM.client.clone();
        let res: Vec<serde_json::Value> = client.call("execute", cmd)?;
        Ok(res)
    }

    /// evaluates an expression in vim and immediately returns.
    pub fn call(&self, cmd: EvalParams) -> Fallible<()> {
        let client = VIM.client.clone();
        client.notify("call", cmd)?;
        Ok(())
    }

    fn set_signs(&self, list: Vec<Sign>) -> Fallible<()> {
        let client = VIM.client.clone();
        client.notify("setSigns", list)?;
        Ok(())
    }

    fn set_quickfix(&self, list: Vec<QuickfixItem>) -> Fallible<()> {
        let client = VIM.client.clone();
        client.notify("setQuickfix", list)?;
        Ok(())
    }

    pub fn show_message(&self, message: Message) -> Fallible<()> {
        let client = VIM.client.clone();
        client.notify("showMessage", message)?;
        Ok(())
    }

    pub fn log_message(&self, params: lsp_types::LogMessageParams) -> Fallible<()> {
        log::debug!("{}", params.message);
        Ok(())
    }
}
