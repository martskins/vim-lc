use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvalParams {
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct QuickfixItem {
    pub bufnr: u64,
    pub filename: String,
    pub lnum: u64,
    pub col: u64,
    pub text: String,
    pub kind: char,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub filename: String,
    pub line: u64,
    pub col: u64,
}

impl From<lsp_types::LocationLink> for Location {
    fn from(f: lsp_types::LocationLink) -> Self {
        Location {
            filename: f.target_uri.to_string(),
            line: f.target_range.start.line + 1,
            col: f.target_range.start.character + 1,
        }
    }
}

impl From<lsp_types::Location> for Location {
    fn from(f: lsp_types::Location) -> Self {
        Location {
            filename: f.uri.to_string(),
            line: f.range.start.line + 1,
            col: f.range.start.character + 1,
        }
    }
}

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
