use braint_proto::{JsonRpcRequest, JsonRpcResponse, SubscribeRequest, SubscribeResponse, SubscriptionId, METHOD_SUBSCRIBE};
use interprocess::local_socket::tokio::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};

struct OutboundRequest {
    id: i64,
    bytes: Vec<u8>,
    reply_tx: oneshot::Sender<Vec<u8>>,
}

pub struct Client {
    next_id: Arc<AtomicI64>,
    write_tx: mpsc::Sender<OutboundRequest>,
    sub_registry: Arc<Mutex<HashMap<String, mpsc::Sender<Vec<u8>>>>>,
    _shutdown: broadcast::Sender<()>,
}

impl Client {
    pub async fn connect(path: &str) -> crate::error::Result<Self> {
        use interprocess::local_socket::GenericFilePath;
        let name = path
            .to_fs_name::<GenericFilePath>()
            .map_err(|e| crate::error::ClientError::DaemonUnreachable(e.to_string()))?;
        let stream = LocalSocketStream::connect(name)
            .await
            .map_err(|e| crate::error::ClientError::DaemonUnreachable(e.to_string()))?;

        let (read_half, write_half) = stream.split();

        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Vec<u8>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let sub_registry: Arc<Mutex<HashMap<String, mpsc::Sender<Vec<u8>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        let (write_tx, write_rx) = mpsc::channel::<OutboundRequest>(64);

        // Spawn writer task
        let pending_for_writer = Arc::clone(&pending);
        let shutdown_for_writer = shutdown_tx.subscribe();
        tokio::spawn(writer_task(write_half, write_rx, pending_for_writer, shutdown_for_writer));

        // Spawn reader task
        let pending_for_reader = Arc::clone(&pending);
        let sub_registry_for_reader = Arc::clone(&sub_registry);
        let shutdown_for_reader = shutdown_tx.subscribe();
        tokio::spawn(reader_task(
            read_half,
            pending_for_reader,
            sub_registry_for_reader,
            shutdown_for_reader,
        ));

        Ok(Self {
            next_id: Arc::new(AtomicI64::new(1)),
            write_tx,
            sub_registry,
            _shutdown: shutdown_tx,
        })
    }

    pub async fn send<Req, Resp>(
        &self,
        method: &str,
        params: &Req,
    ) -> crate::error::Result<Resp>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params: serde_json::to_value(params).map_err(crate::error::ClientError::Serde)?,
        };
        let bytes = serde_json::to_vec(&request).map_err(crate::error::ClientError::Serde)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        self.write_tx
            .send(OutboundRequest { id, bytes, reply_tx })
            .await
            .map_err(|_| {
                crate::error::ClientError::DaemonUnreachable("connection closed".into())
            })?;
        let response_bytes = reply_rx.await.map_err(|_| {
            crate::error::ClientError::DaemonUnreachable("reply channel closed".into())
        })?;
        let response: JsonRpcResponse<Resp> = serde_json::from_slice(&response_bytes)
            .map_err(crate::error::ClientError::Serde)?;
        match response.result {
            Some(r) => Ok(r),
            None => {
                let msg = response.error.map(|e| e.message).unwrap_or_default();
                Err(crate::error::ClientError::DaemonError(msg))
            }
        }
    }

    /// Send a subscribe request and return the subscription ID and a channel for notifications.
    pub async fn subscribe(
        &self,
        request: &SubscribeRequest,
    ) -> crate::error::Result<(SubscriptionId, mpsc::Receiver<Vec<u8>>)> {
        let resp: SubscribeResponse = self.send(METHOD_SUBSCRIBE, request).await?;
        let sub_id = resp.subscription_id;

        let (tx, rx) = mpsc::channel(256);
        self.sub_registry
            .lock()
            .await
            .insert(sub_id.0.to_string(), tx);

        Ok((sub_id, rx))
    }
}

// Re-export read/write frame from the framing module for use in tasks below.
use crate::framing::{read_frame, write_frame};

type ReadHalf = interprocess::local_socket::tokio::RecvHalf;
type WriteHalf = interprocess::local_socket::tokio::SendHalf;

async fn reader_task(
    mut read_half: ReadHalf,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Vec<u8>>>>>,
    subs: Arc<Mutex<HashMap<String, mpsc::Sender<Vec<u8>>>>>,
    mut shutdown: broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            frame = read_frame(&mut read_half) => {
                match frame {
                    Err(_) => break,
                    Ok(bytes) => {
                        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            if v.get("id").is_some() {
                                // It's a response — route by id
                                if let Some(id) = v["id"].as_i64() {
                                    let mut p = pending.lock().await;
                                    if let Some(tx) = p.remove(&id) {
                                        let _ = tx.send(bytes);
                                    }
                                }
                            } else {
                                // It's a notification — route by subscription_id in params
                                if let Some(sub_id) = v["params"]["subscription_id"].as_str() {
                                    let subs = subs.lock().await;
                                    if let Some(tx) = subs.get(sub_id) {
                                        let _ = tx.try_send(bytes);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ = shutdown.recv() => break,
        }
    }
}

async fn writer_task(
    mut write_half: WriteHalf,
    mut rx: mpsc::Receiver<OutboundRequest>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Vec<u8>>>>>,
    mut shutdown: broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            req = rx.recv() => {
                match req {
                    None => break,
                    Some(req) => {
                        pending.lock().await.insert(req.id, req.reply_tx);
                        if write_frame(&mut write_half, &req.bytes).await.is_err() {
                            break;
                        }
                    }
                }
            }
            _ = shutdown.recv() => break,
        }
    }
}
