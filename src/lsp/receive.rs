use crate::language_client::LanguageClient;
use crate::rpc;
use crate::vim;
use failure::Fallible;
use lsp_types::*;

impl LanguageClient<rpc::Client> {
    pub fn progress(&self, params: lsp_types::ProgressParams) -> Fallible<()> {
        let message = match params.value {
            ProgressParamsValue::WorkDone(wd) => match wd {
                WorkDoneProgress::Begin(r) => {
                    Some(format!("{} {}", r.title, r.message.unwrap_or_default()))
                }
                WorkDoneProgress::Report(r) => r.message,
                WorkDoneProgress::End(r) => r.message,
            },
        };

        if message.is_none() {
            return Ok(());
        }

        let message = vim::Message {
            message: message.unwrap(),
            level: vim::LogLevel::Info,
        };

        self.show_message(message)?;
        Ok(())
    }

    pub fn window_show_message(&self, input: ShowMessageParams) -> Fallible<()> {
        let message = input.message;
        self.show_message(vim::Message {
            message,
            level: vim::LogLevel::Info,
        })?;

        Ok(())
    }

    pub fn text_document_publish_diagnostics(
        &self,
        input: PublishDiagnosticsParams,
    ) -> Fallible<()> {
        if input.diagnostics.is_empty() {
            return Ok(());
        }

        let uri = input.uri.to_string();
        let diagnostics = input
            .diagnostics
            .into_iter()
            .map(|d| vim::Diagnostic {
                text_document: uri.clone(),
                line: d.range.start.line + 1,
                col: d.range.start.character + 1,
                text: d.message,
                severity: d.severity.unwrap_or(DiagnosticSeverity::Warning),
            })
            .collect();

        self.show_diagnostics(diagnostics)?;
        Ok(())
    }
}
