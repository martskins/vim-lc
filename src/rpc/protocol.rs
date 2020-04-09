use failure::Fallible;
use jsonrpc_core::Params;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

pub trait RPCClient {
    fn get_reader(&self) -> crossbeam::Receiver<Message>;
    fn reply_success(&self, id: &jsonrpc_core::Id, message: serde_json::Value) -> Fallible<()>;
    fn call<M, R>(&self, method: &str, message: M) -> Fallible<R>
    where
        M: Serialize,
        R: DeserializeOwned;
    fn notify<M>(&self, method: &str, message: M) -> Fallible<()>
    where
        M: Serialize;
}

#[derive(Debug, PartialEq, Clone)]
pub enum ServerID {
    VIM,
    LanguageServer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    MethodCall(jsonrpc_core::MethodCall),
    Notification(jsonrpc_core::Notification),
    Output(jsonrpc_core::Output),
}

impl Message {
    pub fn id(&self) -> jsonrpc_core::Id {
        match self {
            Message::MethodCall(msg) => msg.id.clone(),
            Message::Notification(_) => jsonrpc_core::Id::Null,
            Message::Output(msg) => match msg {
                jsonrpc_core::Output::Success(msg) => msg.id.clone(),
                jsonrpc_core::Output::Failure(msg) => msg.id.clone(),
            },
        }
    }
}

pub trait ToParams {
    fn to_params(self) -> Fallible<Params>;
}

impl<T> ToParams for T
where
    T: Serialize,
{
    fn to_params(self) -> Fallible<Params> {
        let json_value = serde_json::to_value(self)?;

        let params = match json_value {
            Value::Null => Params::None,
            Value::Bool(_) | Value::Number(_) | Value::String(_) => Params::Array(vec![json_value]),
            Value::Array(vec) => Params::Array(vec),
            Value::Object(map) => Params::Map(map),
        };

        Ok(params)
    }
}
