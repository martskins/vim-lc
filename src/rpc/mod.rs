mod protocol;

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
pub use protocol::*;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Debug)]
pub struct Client {
    server_id: ClientID,
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

fn loop_write<O>(server_id: ClientID, mut writer: O, receiver: Receiver<Message>) -> Result<()>
where
    O: Write + Unpin + Send + 'static,
{
    for message in receiver.iter() {
        let message = serde_json::to_string(&message)?;
        log::debug!(
            "{:?} [thread: {:?}] <== {}\n",
            server_id,
            std::thread::current().id(),
            message
        );
        let message = message + "\r\n";

        let message = message.as_bytes();
        let headers = format!("Content-Length: {}\r\n\r\n", message.len());

        writer.write_all(headers.as_bytes())?;
        writer.write_all(message)?;
        writer.flush()?;
    }

    Ok(())
}

fn loop_read<I>(
    server_id: ClientID,
    mut reader: I,
    pending_receiver: Receiver<(jsonrpc_core::Id, Sender<jsonrpc_core::Output>)>,
    sender: Sender<Message>,
) -> Result<()>
where
    I: BufRead + Unpin + Send + 'static,
{
    let mut pending_outputs = HashMap::new();
    loop {
        let mut content_length = String::new();
        reader.read_line(&mut content_length)?;
        let content_length: String = content_length.trim().split(':').skip(1).take(1).collect();
        let content_length = content_length.trim().parse()?;

        let mut content_type = String::new();
        reader.read_line(&mut content_type)?;

        let mut message = vec![0 as u8; content_length];
        reader.read_exact(&mut message)?;
        let message = String::from_utf8(message)?;
        log::debug!(
            "{:?} [thread: {:?}] ==> {}",
            server_id,
            std::thread::current().id(),
            message
        );

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
    fn new<I, O>(server_id: ClientID, reader: I, writer: O) -> Self
    where
        I: BufRead + Unpin + Send + 'static,
        O: Write + Unpin + Send + 'static,
    {
        let (pending_tx, pending_rx) = crossbeam::channel::bounded(1);
        let (reader_tx, reader_rx) = crossbeam::channel::unbounded();
        {
            let server_id = server_id.clone();
            std::thread::spawn(move || {
                if let Err(e) = loop_read(server_id, reader, pending_rx, reader_tx) {
                    log::error!("{}", e);
                }
            });
            // std::thread::spawn(move || {
            //     let mut rt = tokio::runtime::Runtime::new().unwrap();
            //     rt.block_on(loop_read(server_id, reader, pending_rx, reader_tx))
            //         .unwrap();
            // });
        }

        let (writer_tx, writer_rx) = crossbeam::channel::bounded(1);
        {
            let server_id = server_id.clone();
            std::thread::spawn(move || {
                if let Err(e) = loop_write(server_id, writer, writer_rx) {
                    log::error!("{}", e);
                }
            });
            // std::thread::spawn(move || {
            //     let mut rt = tokio::runtime::Runtime::new().unwrap();
            //     rt.block_on(loop_write(server_id, writer, writer_rx))
            //         .unwrap();
            // });
        }

        Self {
            server_id,
            reader_rx,
            writer_tx,
            pending_tx,
            id: Arc::new(AtomicU64::default()),
        }
    }

    fn reply_success(
        &self,
        message_id: &jsonrpc_core::Id,
        message: serde_json::Value,
    ) -> Result<()> {
        let message = jsonrpc_core::Output::Success(jsonrpc_core::Success {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            result: serde_json::to_value(message)?,
            id: message_id.clone(),
        });

        self.writer_tx.send(Message::Output(message))?;
        Ok(())
    }

    fn get_reader(&self) -> Receiver<Message> {
        self.reader_rx.clone()
    }

    fn notify<M>(&self, method: &str, message: M) -> Result<()>
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

    fn call<M, R>(&self, method: &str, message: M) -> Result<R>
    where
        M: Serialize,
        R: DeserializeOwned,
    {
        let (tx, rx) = crossbeam::channel::bounded(1);
        let id = self.id.fetch_add(1, Ordering::SeqCst);

        let message = jsonrpc_core::MethodCall {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            method: method.into(),
            params: message.to_params()?,
            id: jsonrpc_core::Id::Num(id),
        };

        self.pending_tx.send((jsonrpc_core::Id::Num(id), tx))?;
        self.writer_tx.send(Message::MethodCall(message))?;

        let message = rx.recv()?;
        match message {
            jsonrpc_core::Output::Success(s) => Ok(serde_json::from_value(s.result)?),
            jsonrpc_core::Output::Failure(s) => Err(anyhow::anyhow!(s.error)),
        }
    }
}
