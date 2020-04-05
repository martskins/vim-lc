use crossbeam::Sender;
use failure::Fallible;
use jsonrpc_core::Params;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
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
    pub fn body(&self) -> Fallible<String> {
        let value = match self {
            Message::MethodCall(msg) => serde_json::to_string(&msg.params)?,
            Message::Notification(msg) => serde_json::to_string(&msg.params)?,
            Message::Output(msg) => match msg {
                jsonrpc_core::Output::Success(msg) => serde_json::to_string(&msg.result)?,
                jsonrpc_core::Output::Failure(msg) => serde_json::to_string(&msg.error)?,
            },
        };

        Ok(value)
    }

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

    pub fn kind(&self) -> &str {
        match self {
            Message::MethodCall(_) => "method_call",
            Message::Notification(_) => "notification",
            Message::Output(_) => "output",
        }
    }

    pub fn method(&self) -> &str {
        match self {
            Message::MethodCall(msg) => msg.method.as_str(),
            Message::Notification(msg) => msg.method.as_str(),
            Message::Output(_) => "",
        }
    }
}

#[derive(Debug)]
pub struct Client<I, O> {
    server_id: ServerID,
    reader: Arc<Mutex<I>>,
    writer: Arc<Mutex<O>>,
    id: Arc<Mutex<AtomicU64>>,
    pending_responses: Arc<Mutex<HashMap<jsonrpc_core::Id, Sender<jsonrpc_core::Output>>>>,
}

impl<I, O> Clone for Client<I, O> {
    fn clone(&self) -> Client<I, O> {
        Self {
            server_id: self.server_id.clone(),
            reader: self.reader.clone(),
            writer: self.writer.clone(),
            id: self.id.clone(),
            pending_responses: self.pending_responses.clone(),
        }
    }
}

impl<I, O> Client<I, O>
where
    I: AsyncBufReadExt + Unpin,
    O: AsyncWrite + Unpin,
{
    pub fn new(server_id: ServerID, reader: I, writer: O) -> Self {
        Self {
            server_id,
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            id: Arc::new(Mutex::new(AtomicU64::default())),
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // pub fn new(rpcserver: RpcServer, reader: Arc<Mutex<I>>, writer: Arc<Mutex<O>>) -> Self {
    //     Self {
    //         rpcserver,
    //         reader,
    //         writer,
    //         id: Arc::new(Mutex::new(AtomicU64::default())),
    //         pending_responses: Arc::new(Mutex::new(HashMap::new())),
    //     }
    // }

    pub fn cancel(&self, message_id: &jsonrpc_core::Id) -> Fallible<()> {
        let mut pending_responses = self.pending_responses.try_lock()?;
        pending_responses.remove(message_id);
        Ok(())
    }

    // pub async fn reply_failure(
    //     &self,
    //     message_id: &jsonrpc_core::Id,
    //     error: jsonrpc_core::Error,
    // ) -> Fallible<()> {
    //     let msg = jsonrpc_core::Output::Failure(jsonrpc_core::Failure {
    //         jsonrpc: Some(jsonrpc_core::Version::V2),
    //         error,
    //         id: message_id.clone(),
    //     });

    //     self.send_raw(msg).await?;
    //     Ok(())
    // }

    pub async fn reply_success(
        &mut self,
        message_id: &jsonrpc_core::Id,
        message: serde_json::Value,
    ) -> Fallible<()> {
        let msg = jsonrpc_core::Output::Success(jsonrpc_core::Success {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            result: serde_json::to_value(message)?,
            id: message_id.clone(),
        });

        self.send_raw(msg).await?;
        Ok(())
    }

    pub async fn resolve(
        &self,
        message_id: &jsonrpc_core::Id,
        message: jsonrpc_core::Output,
    ) -> Fallible<()> {
        let mut pending_responses = self.pending_responses.try_lock()?;
        if let Some(tx) = pending_responses.remove(message_id) {
            tx.send(message)?;
        }
        Ok(())
    }

    pub async fn read(&mut self) -> Fallible<Message> {
        let mut reader = self.reader.try_lock()?;

        let mut content_length = String::new();
        reader.read_line(&mut content_length).await?;
        log::error!("{}", content_length);
        let content_length: String = content_length.trim().split(':').skip(1).take(1).collect();
        let content_length = content_length.trim().parse()?;

        let mut content_type = String::new();
        reader.read_line(&mut content_type).await?;
        log::error!("{}", content_type);

        let mut message = vec![0 as u8; content_length];
        reader.read_exact(&mut message).await?;
        let message = String::from_utf8(message)?;
        log::error!("{:?} ==> {}\n", self.server_id, message);

        let message = serde_json::from_str(message.as_str())?;
        Ok(message)
    }

    pub async fn notify<M>(&mut self, method: &str, message: M) -> Fallible<()>
    where
        M: Serialize,
    {
        let message = jsonrpc_core::Notification {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            method: method.into(),
            params: message.to_params()?,
        };

        self.send_raw(message).await?;
        Ok(())
    }

    pub async fn call<M>(&mut self, method: &str, message: M) -> Fallible<u64>
    where
        M: Serialize,
    {
        let id_lock = self.id.try_lock()?;
        let id = id_lock.fetch_add(1, Ordering::SeqCst);
        drop(id_lock);

        let message = jsonrpc_core::MethodCall {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            method: method.into(),
            params: message.to_params()?,
            id: jsonrpc_core::Id::Num(id),
        };

        self.send_raw(message).await?;
        Ok(id)
    }

    async fn send_raw<M: Serialize>(&mut self, message: M) -> Fallible<()> {
        let message = serde_json::to_string(&message)?;
        log::error!("{:?} <== {}\n", self.server_id, message);
        // let message = message + "\r\n";

        let message = message.as_bytes();
        let headers = format!("Content-Length: {}\r\n\r\n", message.len());

        let mut writer = self.writer.try_lock()?;
        writer.write_all(headers.as_bytes()).await?;
        writer.write_all(message).await?;

        Ok(())
    }

    pub async fn call_and_wait<M, R>(&mut self, method: &str, message: M) -> Fallible<R>
    where
        M: Serialize + std::fmt::Debug + Clone,
        R: DeserializeOwned,
    {
        let (tx, rx) = crossbeam::bounded(1);

        let id = self.call(method, message.clone()).await?;
        let mut pending_responses = self.pending_responses.try_lock()?;
        pending_responses.insert(jsonrpc_core::Id::Num(id), tx);
        drop(pending_responses);

        let message = rx.recv()?;
        match message {
            jsonrpc_core::Output::Success(s) => Ok(serde_json::from_value(s.result)?),
            jsonrpc_core::Output::Failure(s) => failure::bail!(s.error),
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
