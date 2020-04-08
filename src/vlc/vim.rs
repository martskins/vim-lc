use super::VLC;
use crate::rpc::RPCClient;
use crate::vim::*;
use crate::LANGUAGE_CLIENT;
use crate::VIM;
use failure::Fallible;

impl<T> VLC<T>
where
    T: RPCClient,
{
    pub fn apply_edits(&self, edits: lsp_types::WorkspaceEdit) -> Fallible<()> {
        let changes = self.text_document_changes(edits)?;
        self.client.notify("applyEdits", changes)?;
        Ok(())
    }

    async fn text_document_edits(
        &self,
        filename: &str,
        edits: lsp_types::TextDocumentEdit,
    ) -> Fallible<Vec<Lines>> {
        let tasks: Vec<_> = edits
            .edits
            .into_iter()
            .map(|e| self.text_document_edit(filename, e))
            .collect();

        let out = futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter(Result::is_ok)
            .map(|c| c.unwrap())
            .collect();
        Ok(out)
    }

    async fn text_document_edit(
        &self,
        filename: &str,
        edit: lsp_types::TextEdit,
    ) -> Fallible<Lines> {
        let mut line: String = LANGUAGE_CLIENT
            .get_line(filename, edit.range.start.line + 1)
            .await?;

        let start = edit.range.start.character as usize;
        let end = edit.range.end.character as usize;
        line.replace_range(start..end, &edit.new_text);

        let lines = vec![Line {
            line: edit.range.start.line + 1,
            text: line,
        }];

        Ok(Lines { lines })
    }

    async fn text_document_change_from_edit(
        &self,
        edit: lsp_types::TextDocumentEdit,
    ) -> Fallible<TextDocumentChanges> {
        let text_document = edit
            .text_document
            .uri
            .to_string()
            .replace(self.root_path.as_str(), "");
        Ok(TextDocumentChanges {
            text_document: text_document.clone(),
            edits: self.text_document_edits(&text_document, edit).await?,
        })
    }

    fn text_document_changes(
        &self,
        f: lsp_types::WorkspaceEdit,
    ) -> Fallible<Vec<TextDocumentChanges>> {
        let document_changes = f
            .document_changes
            .unwrap_or_else(|| lsp_types::DocumentChanges::Edits(vec![]));

        use futures::executor::block_on;
        let changes = match document_changes {
            lsp_types::DocumentChanges::Edits(edits) => edits
                .into_iter()
                .map(|v| block_on(self.text_document_change_from_edit(v)))
                .filter(Result::is_ok)
                .map(|c| c.unwrap())
                .collect(),
            lsp_types::DocumentChanges::Operations(operations) => vec![],
        };

        Ok(changes)
    }

    pub fn show_diagnostics(&self, mut diagnostics: Vec<Diagnostic>) -> Fallible<()> {
        diagnostics.iter_mut().for_each(|d| {
            d.text_document = d.text_document.replace(self.root_path.as_str(), "");
        });

        let quickfix_list: Vec<QuickfixItem> =
            diagnostics.clone().into_iter().map(|l| l.into()).collect();
        self.set_quickfix(quickfix_list)?;

        let signs: Vec<Sign> = diagnostics.into_iter().map(|l| l.into()).collect();
        self.set_signs(signs)?;

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

        let text = match input.contents {
            lsp_types::HoverContents::Scalar(ref c) => match c.clone() {
                lsp_types::MarkedString::String(s) => s,
                lsp_types::MarkedString::LanguageString(s) => s.value,
            },
            lsp_types::HoverContents::Array(ref c) => {
                if c.is_empty() {
                    String::new()
                } else {
                    match c[0].clone() {
                        lsp_types::MarkedString::String(s) => s,
                        lsp_types::MarkedString::LanguageString(s) => s.value,
                    }
                }
            }
            lsp_types::HoverContents::Markup(c) => c.value,
        };

        let client = VIM.client.clone();
        client.notify("showPreview", PreviewContent { filetype, text })?;
        Ok(())
    }

    pub fn show_locations(&self, input: Vec<Location>) -> Fallible<()> {
        use futures::executor::block_on;

        if input.is_empty() {
            return Ok(());
        }

        if input.len() == 1 {
            return self.jump_to_location(input.first().cloned().unwrap());
        }

        let list = input
            .into_iter()
            .map(|l| {
                let filename = l.filename.replace(self.root_path.as_str(), "");
                // TODO: parallelize these calls
                let text = block_on(LANGUAGE_CLIENT.get_line(filename.as_str(), l.position.line))
                    .unwrap_or_default();

                QuickfixItem {
                    bufnr: 0,
                    filename: l.filename.replace(self.root_path.as_str(), ""),
                    line: l.position.line,
                    column: l.position.column,
                    text,
                    kind: 'W',
                }
            })
            .collect();

        self.set_quickfix(list)?;
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
    fn execute(&self, cmd: Vec<ExecuteParams>) -> Fallible<Vec<serde_json::Value>> {
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
