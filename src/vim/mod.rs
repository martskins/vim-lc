use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub level: u64,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct BaseParams {
    pub bufnr: u64,
    pub language_id: String,
}

#[derive(Debug, Deserialize)]
pub struct TextDocumentPosition {
    pub text_document: String,
    pub line: u64,
    pub column: u64,
    pub language_id: String,
}

impl Into<lsp_types::TextDocumentPositionParams> for TextDocumentPosition {
    fn into(self) -> lsp_types::TextDocumentPositionParams {
        lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Url::from_file_path(self.text_document).unwrap(),
            },
            position: lsp_types::Position {
                line: self.line,
                character: self.column,
            },
        }
    }
}
