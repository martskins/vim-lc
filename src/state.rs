use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct State {
    pub process_id: u32,
    // contains file uris as a key and a tuple of (Version, Text) as a value.
    pub text_documents: HashMap<String, (u64, Vec<String>)>,
    pub server_capabilities: HashMap<String, lsp_types::ServerCapabilities>,
    // when a user calls textDocument/codeAction actions are fetched from the server and stashed in
    // this vec for later resolution.
    pub code_actions: Vec<lsp_types::CodeActionOrCommand>,
    // when textDocument/codeLens is resolved, we insert the result in this hashmap where the key
    // is the name of the text document. This hashmap will be used to fetch the code lens actions
    // in a specific line and file.
    pub code_lens: HashMap<String, Vec<lsp_types::CodeLens>>,
}
