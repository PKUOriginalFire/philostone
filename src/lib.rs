use std::{
    net::{TcpListener, TcpStream},
    sync::Arc,
};

use async_broadcast::{broadcast, Receiver, Sender};
use async_tungstenite::WebSocketStream;
use compact_str::CompactString;
use futures::{sink::SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use smol::{future::FutureExt, Async};
use thiserror::Error;
use tungstenite::Message;

use crate::pool::{DefaultPool, PoolKey};

pub mod pool;

/// A danmaku.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Danmaku {
    /// The danmaku's sender.
    pub sender: CompactString,
    /// The danmaku's text.
    pub text: CompactString,
    /// The danmaku's font color.
    pub color: CompactString,
    /// The danmaku's font size.
    pub size: u32,
}

/// An error that can occur while handling a client connection.
#[derive(Debug, Error)]
pub enum DanmakuError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

/// Handles a client connection.
pub async fn handle(
    pool: Arc<DefaultPool>,
    stream: WebSocketStream<Async<TcpStream>>,
    broadcast_tx: Sender<PoolKey>,
    mut broadcast_rx: Receiver<PoolKey>,
) -> Result<(), DanmakuError> {
    let (mut writer, mut reader) = stream.split();

    // Receive messages from the client and broadcast them to all other clients.
    let recv = async {
        while let Some(msg) = reader.next().await {
            let text = match msg?.into_text() {
                Ok(text) => text,
                Err(e) => {
                    tracing::warn!("non-text message: {}", e);
                    continue;
                }
            };
            let danmaku = match serde_json::from_str::<Danmaku>(&text) {
                Ok(danmaku) => danmaku,
                Err(e) => {
                    tracing::warn!("invalid json data: {}", e);
                    continue;
                }
            };
            tracing::info!("received danmaku: {:?}", danmaku);

            let key = pool.write().await.insert(danmaku);
            if let Err(e) = broadcast_tx.broadcast_direct(key).await {
                tracing::warn!("failed to broadcast: {}", e);
            }
            tracing::debug!("broadcasted danmaku: {:?}", key);
        }
        Ok(())
    };

    // Send messages to the client as they are broadcasted.
    let send = async {
        while let Ok(key) = broadcast_rx.recv_direct().await {
            tracing::debug!("sending danmaku: {:?}", key);

            let pool = pool.read().await;
            let danmaku = match pool.get(key) {
                Some(danmaku) => danmaku,
                None => {
                    tracing::warn!("missing danmaku: {:?}", key);
                    continue;
                }
            };

            writer
                .send(Message::text(serde_json::to_string(danmaku)?))
                .await?;
        }
        Ok(())
    };

    recv.race(send).await
}

/// Listens for incoming connections and serves them.
pub async fn listen(listener: Async<TcpListener>) -> Result<(), DanmakuError> {
    let host = format!("ws://{}", listener.get_ref().local_addr()?);
    println!("Listening on {}", host);

    let pool: Arc<DefaultPool> = Default::default();
    let (tx, rx) = broadcast(20);

    loop {
        // Accept the next connection.
        let (stream, _) = listener.accept().await?;
        println!("Accepted client: {}", stream.get_ref().peer_addr()?);

        // let stream = WsStream::Plain(async_tungstenite::accept_async(stream).await?);
        let stream = async_tungstenite::accept_async(stream).await?;
        smol::spawn(handle(pool.clone(), stream, tx.clone(), rx.clone())).detach();
    }
}
