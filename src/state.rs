use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct State {
    pub process_id: u64,
    // contains file uris as a key and a tuple of (Version, Text) as a value.
    pub text_documents: HashMap<String, (u64, Vec<String>)>,
    pub server_capabilities: HashMap<String, lsp_types::ServerCapabilities>,
    pub code_actions: Vec<lsp_types::CodeActionOrCommand>,
}
