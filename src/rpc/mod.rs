mod protocol;

use crossbeam::{Receiver, Sender};
use failure::Fallible;
pub use protocol::*;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct Client {
    server_id: ServerID,
    reader_rx: Receiver<Message>,
    writer_tx: Sender<Message>,
    pending_tx: Sender<(jsonrpc_core::Id, Sender<jsonrpc_core::Output>)>,
    id: Arc<AtomicU64>,
}

impl Clone for Client {
    fn clone(&self) -> Client {
        Self {
            server_id: self.server_id.clone(),
            reader_rx: self.reader_rx.clone(),
            writer_tx: self.writer_tx.clone(),
            pending_tx: self.pending_tx.clone(),
            id: self.id.clone(),
        }
    }
}

impl Client {
    pub fn new<I, O>(server_id: ServerID, reader: I, writer: O) -> Self
    where
        I: AsyncBufReadExt + Unpin + Send + 'static,
        O: AsyncWrite + Unpin + Send + 'static,
    {
        let (pending_tx, pending_rx) = crossbeam::unbounded();
        let (reader_tx, reader_rx) = crossbeam::unbounded();
        {
            let server_id = server_id.clone();
            tokio::spawn(async move {
                loop_read::<I>(server_id, reader, pending_rx, reader_tx)
                    .await
                    .unwrap();
            });
        }

        let (writer_tx, writer_rx) = crossbeam::unbounded();
        {
            let server_id = server_id.clone();
            tokio::spawn(async move {
                loop_write::<O>(server_id, writer, writer_rx).await.unwrap();
            });
        }

        Self {
            server_id,
            reader_rx,
            writer_tx,
            pending_tx,
            id: Arc::new(AtomicU64::default()),
        }
    }
}

async fn loop_write<O>(
    server_id: ServerID,
    mut writer: O,
    receiver: Receiver<Message>,
) -> Fallible<()>
where
    O: AsyncWrite + Unpin + Send + 'static,
{
    for message in receiver.iter() {
        let message = serde_json::to_string(&message)?;
        log::error!("{:?} <== {}\n", server_id, message);
        let message = message + "\r\n";

        let message = message.as_bytes();
        let headers = format!("Content-Length: {}\r\n\r\n", message.len());

        writer.write_all(headers.as_bytes()).await?;
        writer.write_all(message).await?;
        writer.flush().await?;
    }

    Ok(())
}

async fn loop_read<I>(
    server_id: ServerID,
    mut reader: I,
    pending_receiver: Receiver<(jsonrpc_core::Id, Sender<jsonrpc_core::Output>)>,
    sender: Sender<Message>,
) -> Fallible<()>
where
    I: AsyncBufReadExt + Unpin + Send + 'static,
{
    let mut pending_outputs = HashMap::new();
    loop {
        let mut content_length = String::new();
        reader.read_line(&mut content_length).await?;
        let content_length: String = content_length.trim().split(':').skip(1).take(1).collect();
        let content_length = content_length.trim().parse()?;

        let mut content_type = String::new();
        reader.read_line(&mut content_type).await?;

        let mut message = vec![0 as u8; content_length];
        reader.read_exact(&mut message).await?;
        let message = String::from_utf8(message)?;
        log::error!("{:?} ==> {}\n", server_id, message);

        let message: Message = serde_json::from_str(message.as_str())?;
        let message_id = message.id();
        match message {
            Message::Output(output) => {
                while let Ok((id, tx)) = pending_receiver.try_recv() {
                    pending_outputs.insert(id, tx);
                }

                if let Some(tx) = pending_outputs.remove(&message_id) {
                    tx.send(output)?;
                }
            }
            _ => {
                sender.send(message.clone())?;
            }
        }
    }
}

impl RPCClient for Client {
    fn reply_success(
        &self,
        message_id: &jsonrpc_core::Id,
        message: serde_json::Value,
    ) -> Fallible<()> {
        let message = jsonrpc_core::Output::Success(jsonrpc_core::Success {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            result: serde_json::to_value(message)?,
            id: message_id.clone(),
        });

        self.writer_tx.send(Message::Output(message))?;
        Ok(())
    }

    fn read(&self) -> Fallible<Message> {
        let message = self.reader_rx.recv()?;
        Ok(message)
    }

    fn notify<M>(&self, method: &str, message: M) -> Fallible<()>
    where
        M: Serialize,
    {
        let message = jsonrpc_core::Notification {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            method: method.into(),
            params: message.to_params()?,
        };

        self.writer_tx.send(Message::Notification(message))?;
        Ok(())
    }

    fn call<M, R>(&self, method: &str, message: M) -> Fallible<R>
    where
        M: Serialize,
        R: DeserializeOwned,
    {
        let (tx, rx) = crossbeam::bounded(1);
        let id = self.id.fetch_add(1, Ordering::SeqCst);

        let message = jsonrpc_core::MethodCall {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            method: method.into(),
            params: message.to_params()?,
            id: jsonrpc_core::Id::Num(id),
        };

        self.writer_tx.send(Message::MethodCall(message))?;
        self.pending_tx.send((jsonrpc_core::Id::Num(id), tx))?;

        let message = rx.recv()?;
        match message {
            jsonrpc_core::Output::Success(s) => Ok(serde_json::from_value(s.result)?),
            jsonrpc_core::Output::Failure(s) => failure::bail!(s.error),
        }
    }
}
