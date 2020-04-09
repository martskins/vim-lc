use serde::{Deserialize, Serialize};

// FZFItem is a trait that enables a type to be displayed in FZF.
pub trait FZFItem {
    // user selection callback. This should take the name of a scope local function defined in
    // vim.
    fn sink() -> String;
    // text to display for the item in FZF.
    fn text(&self) -> String;
}

impl FZFItem for lsp_types::CodeLens {
    fn sink() -> String {
        "s:resolveCodeAction".into()
    }

    fn text(&self) -> String {
        format!(
            "{}: {}",
            self.command.as_ref().unwrap().command,
            self.command.as_ref().unwrap().title
        )
    }
}

#[derive(Debug, Serialize)]
pub struct LocationWithPreview {
    pub location: Location,
    pub preview: String,
}

impl FZFItem for LocationWithPreview {
    fn sink() -> String {
        "s:fzfLocationSink".into()
    }

    fn text(&self) -> String {
        format!(
            "{}:{} \t{}",
            self.location.filename, self.location.position.line, self.preview
        )
    }
}

impl FZFItem for Action {
    fn sink() -> String {
        "s:resolveCodeAction".into()
    }

    fn text(&self) -> String {
        format!("{}", self.text)
    }
}

#[derive(Debug, Serialize)]
pub enum HLGroup {
    Comment,
}

#[derive(Debug, Serialize)]
pub struct DocumentChanges {
    pub text_document: String,
    pub changes: Vec<BufChanges>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ResolveCodeActionParams {
    pub selection: String,
    #[serde(flatten)]
    pub position: CursorPosition,
}

#[derive(Debug, Clone, Serialize)]
pub struct Action {
    pub text: String,
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct TextDocumentChanges {
    pub text_document: String,
    pub edits: Vec<Lines>,
}

#[derive(Debug, Default, Serialize)]
pub struct WorkspaceChanges {
    pub changes: Vec<TextDocumentChanges>,
}

#[derive(Debug, Serialize)]
pub struct BufChanges {
    pub start: Position,
    pub end: Position,
    pub lines: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Lines {
    pub lines: Vec<Line>,
}

#[derive(Debug, Serialize)]
pub struct Line {
    pub line: u64,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameParams {
    pub new_name: String,
    #[serde(flatten)]
    pub position: CursorPosition,
}

#[derive(Debug, Serialize)]
pub struct VirtualText {
    pub text: String,
    pub line: u64,
    pub hl_group: HLGroup,
}

#[derive(Debug, Serialize)]
pub struct CompletionList {
    pub words: Vec<CompletionItem>,
}

#[derive(Debug, Default, Serialize)]
pub struct CompletionItem {
    // text that will be inserted, mandatory
    pub word: String,
    // abbreviation of "word"; when not empty it is used in the menu instead of "word"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbr: Option<String>,
    //  extra text for the popup menu, displayed after "word" or "abbr"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu: Option<String>,
    // more information about the item, can be displayed in a preview window
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<String>,
    // single letter indicating the type of completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<char>,
    // when non-zero case is to be ignored when comparing items to be equal; when omitted zero is
    // used, thus items that only differ in case are added
    pub icase: u8,
    // when non-zero, always treat this item to be equal when comparing. Which means, "equal=1"
    // disables filtering of this item.
    pub equal: u8,
    // when non-zero this match will be added even when an item with the same word is already present.
    pub dup: u8,
    // when non-zero this match will be added even when it is an empty string
    pub empty: u8,
}

pub fn completion_item_kind(input: Option<lsp_types::CompletionItemKind>) -> Option<char> {
    match input? {
        lsp_types::CompletionItemKind::Text => None,
        lsp_types::CompletionItemKind::Method => Some('m'),
        lsp_types::CompletionItemKind::Function => Some('f'),
        lsp_types::CompletionItemKind::Constructor => Some('f'),
        lsp_types::CompletionItemKind::Field => Some('p'),
        lsp_types::CompletionItemKind::Variable => Some('v'),
        lsp_types::CompletionItemKind::Class => Some('c'),
        lsp_types::CompletionItemKind::Interface => Some('i'),
        lsp_types::CompletionItemKind::Module => Some('m'),
        lsp_types::CompletionItemKind::Property => Some('p'),
        lsp_types::CompletionItemKind::Unit => None,
        lsp_types::CompletionItemKind::Value => None,
        lsp_types::CompletionItemKind::Enum => Some('e'),
        lsp_types::CompletionItemKind::Keyword => Some('k'),
        lsp_types::CompletionItemKind::Snippet => Some('s'),
        lsp_types::CompletionItemKind::Color => None,
        lsp_types::CompletionItemKind::File => None,
        lsp_types::CompletionItemKind::Reference => None,
        lsp_types::CompletionItemKind::Folder => None,
        lsp_types::CompletionItemKind::EnumMember => None,
        lsp_types::CompletionItemKind::Constant => None,
        lsp_types::CompletionItemKind::Struct => None,
        lsp_types::CompletionItemKind::Event => None,
        lsp_types::CompletionItemKind::Operator => None,
        lsp_types::CompletionItemKind::TypeParameter => None,
    }
}

#[derive(Debug, Serialize)]
pub struct PreviewContent {
    pub filetype: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Sign {
    pub id: u64,
    pub line: u64,
    pub file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Diagnostic {
    pub text_document: String,
    pub line: u64,
    pub col: u64,
    pub text: String,
    pub severity: lsp_types::DiagnosticSeverity,
}

impl Into<Sign> for Diagnostic {
    fn into(self) -> Sign {
        Sign {
            // TODO: not sure what id should be, need to check vim docs
            id: 1,
            line: self.line,
            file: self.text_document,
        }
    }
}

impl Into<QuickfixItem> for Diagnostic {
    fn into(self) -> QuickfixItem {
        let mut kind = 'W';
        if self.severity == lsp_types::DiagnosticSeverity::Error {
            kind = 'E';
        }

        QuickfixItem {
            bufnr: 0,
            filename: self.text_document,
            line: self.line,
            column: self.col,
            text: self.text,
            kind,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TextDocumentIdentifier {
    pub text_document: String,
    pub language_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextDocumentContent {
    pub text_document: String,
    pub text: String,
    pub language_id: String,
}

impl Into<TextDocumentIdentifier> for TextDocumentContent {
    fn into(self) -> TextDocumentIdentifier {
        TextDocumentIdentifier {
            text_document: self.text_document,
            language_id: self.language_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvalParams {
    pub command: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecuteParams {
    pub action: String,
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct QuickfixItem {
    pub bufnr: u64,
    pub filename: String,
    #[serde(rename = "lnum")]
    pub line: u64,
    #[serde(rename = "col")]
    pub column: u64,
    pub text: String,
    pub kind: char,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub filename: String,
    #[serde(flatten)]
    pub position: Position,
}

impl From<lsp_types::LocationLink> for Location {
    fn from(f: lsp_types::LocationLink) -> Self {
        Location {
            filename: f.target_uri.to_string(),
            position: Position {
                line: f.target_range.start.line + 1,
                column: f.target_range.start.character + 1,
            },
        }
    }
}

impl From<lsp_types::Location> for Location {
    fn from(f: lsp_types::Location) -> Self {
        Location {
            filename: f.uri.to_string(),
            position: Position {
                line: f.range.start.line + 1,
                column: f.range.start.character + 1,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LogLevel {
    Error = 1,
    Warning = 2,
    Info = 3,
    Log = 4,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct BufInfo {
    pub bufnr: u64,
    pub language_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CursorPosition {
    /// file name of the text document
    pub text_document: String,
    /// language_id for the text document
    pub language_id: String,
    /// position of the cursor
    #[serde(flatten)]
    pub position: Position,
}

impl Into<lsp_types::ReferenceParams> for CursorPosition {
    fn into(self) -> lsp_types::ReferenceParams {
        lsp_types::ReferenceParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: lsp_types::Url::from_file_path(self.text_document).unwrap(),
                },
                position: lsp_types::Position {
                    line: self.position.line - 1,
                    character: self.position.column - 1,
                },
            },
            work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: false,
            },
        }
    }
}

impl Into<lsp_types::TextDocumentPositionParams> for CursorPosition {
    fn into(self) -> lsp_types::TextDocumentPositionParams {
        lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Url::from_file_path(self.text_document).unwrap(),
            },
            position: lsp_types::Position {
                line: self.position.line - 1,
                character: self.position.column - 1,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Into<lsp_types::Range> for Range {
    fn into(self) -> lsp_types::Range {
        lsp_types::Range {
            start: self.start.into(),
            end: self.end.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// line position in a buffer, one-based
    pub line: u64,
    /// column position in a buffer, one-based
    pub column: u64,
}

impl Into<lsp_types::Position> for Position {
    fn into(self) -> lsp_types::Position {
        lsp_types::Position {
            line: self.line,
            character: self.column,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SelectionRange {
    /// file name of the text document
    pub text_document: String,
    /// language_id for the text document
    pub language_id: String,
    /// start position
    pub range: Range,
}
