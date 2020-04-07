mod protocol;

use async_trait::async_trait;
use crossbeam::Sender;
use failure::Fallible;
pub use protocol::*;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;

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

    pub fn cancel(&self, message_id: &jsonrpc_core::Id) -> Fallible<()> {
        let mut pending_responses = self.pending_responses.try_lock()?;
        pending_responses.remove(message_id);
        Ok(())
    }

    async fn send_raw<M: Serialize>(&self, message: M) -> Fallible<()> {
        let message = serde_json::to_string(&message)?;
        log::error!("{:?} <== {}\n", self.server_id, message);
        let message = message + "\r\n";

        let message = message.as_bytes();
        let headers = format!("Content-Length: {}\r\n\r\n", message.len());

        let mut writer = self.writer.try_lock()?;
        writer.write_all(headers.as_bytes()).await?;
        writer.write_all(message).await?;
        writer.flush().await?;

        Ok(())
    }
}

#[async_trait]
impl<I, O> RPCClient for Client<I, O>
where
    I: AsyncBufReadExt + Unpin + Send,
    O: AsyncWrite + Unpin + Send,
{
    async fn reply_success(
        &self,
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

    async fn resolve(
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

    async fn read(&self) -> Fallible<Message> {
        let mut reader = self.reader.try_lock()?;

        let mut content_length = String::new();
        reader.read_line(&mut content_length).await?;
        let content_length: String = content_length.trim().split(':').skip(1).take(1).collect();
        let content_length = content_length.trim().parse()?;

        let mut content_type = String::new();
        reader.read_line(&mut content_type).await?;

        let mut message = vec![0 as u8; content_length];
        reader.read_exact(&mut message).await?;
        let message = String::from_utf8(message)?;
        log::error!("{:?} ==> {}\n", self.server_id, message);

        let message = serde_json::from_str(message.as_str())?;
        Ok(message)
    }

    async fn notify<M>(&self, method: &str, message: M) -> Fallible<()>
    where
        M: Serialize + Send,
    {
        let message = jsonrpc_core::Notification {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            method: method.into(),
            params: message.to_params()?,
        };

        self.send_raw(message).await?;
        Ok(())
    }

    async fn call<M, R>(&self, method: &str, message: M) -> Fallible<R>
    where
        M: Serialize + std::fmt::Debug + Clone + Send,
        R: DeserializeOwned,
    {
        let (tx, rx) = crossbeam::bounded(1);

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
