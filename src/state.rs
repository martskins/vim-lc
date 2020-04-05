use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct State {
    pub process_id: u64,
    pub text_documents: HashMap<String, u64>,
    pub server_capabilities: HashMap<String, lsp_types::ServerCapabilities>,
}
