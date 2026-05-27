// P2P client.
//
// DEVNET v0: a minimal length-prefixed JSON message protocol over TCP. The
// real DOM protocol will replace this with a noise/quic transport. The shape
// here is what the embedded node actually drives: a backbone connection
// supervisor that reconnects, pulls headers/blocks, relays signed
// transactions, and reports back to the chain state and peer registry.
//
// Today: best-effort reachability ping + version handshake. If the backbone
// is unreachable the node still runs locally and produces blocks via mining
// if enabled, deterministically — the chain identity is intact regardless.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::chain::state::ChainState;
use crate::net::peer::PeerRegistry;
use crate::net::{BACKBONE_HOST, BACKBONE_PORT, BACKBONE_PEER};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum P2pMessage {
    Version { protocol: u32, height: u64, address: String, chain_id: String },
    VersionAck { protocol: u32, height: u64 },
    GetHeaders { from_height: u64 },
    Headers { hashes: Vec<String>, heights: Vec<u64> },
    Ping { nonce: u64 },
    Pong { nonce: u64 },
}

pub struct P2pClient {
    chain: Arc<ChainState>,
    peers: Arc<PeerRegistry>,
    address: String,
}

impl P2pClient {
    pub fn new(chain: Arc<ChainState>, peers: Arc<PeerRegistry>, address: String) -> Self {
        Self { chain, peers, address }
    }

    /// Backbone supervisor loop. Tries to keep one healthy connection open at
    /// all times. Backs off exponentially on failure.
    pub async fn supervisor(self: Arc<Self>, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let mut backoff_ms = 1000u64;
        loop {
            if *shutdown.borrow() {
                tracing::info!("p2p supervisor shutting down");
                break;
            }

            tracing::info!(peer = BACKBONE_PEER, "connecting to backbone");
            match self.connect_once().await {
                Ok(_) => {
                    tracing::info!("backbone session ended cleanly");
                    backoff_ms = 1000;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "backbone connection failed");
                    self.peers.note_failure(BACKBONE_PEER);
                    backoff_ms = (backoff_ms * 2).min(60_000);
                }
            }

            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => {}
                _ = shutdown.changed() => {}
            }
        }
    }

    async fn connect_once(&self) -> Result<()> {
        let addr = format!("{BACKBONE_HOST}:{BACKBONE_PORT}");
        let stream = tokio::time::timeout(
            Duration::from_secs(8),
            TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| anyhow::anyhow!("connection timeout"))??;

        let mut stream = stream;

        let version = P2pMessage::Version {
            protocol: 1,
            height: self.chain.height(),
            address: self.address.clone(),
            chain_id: crate::chain::state::DOM_CHAIN_ID.to_string(),
        };
        send_msg(&mut stream, &version).await?;

        // Read VersionAck with a short timeout.
        match tokio::time::timeout(Duration::from_secs(5), recv_msg(&mut stream)).await {
            Ok(Ok(P2pMessage::VersionAck { height, .. })) => {
                self.peers.note_success(BACKBONE_PEER, height);
                tracing::info!(remote_height = height, "backbone handshake complete");
            }
            Ok(Ok(other)) => {
                tracing::warn!(?other, "unexpected message during handshake");
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // No ACK — the backbone may not yet implement this protocol.
                // We still consider the TCP reach a successful liveness check
                // and keep the connection alive briefly for stats.
                self.peers.note_success(BACKBONE_PEER, 0);
                tracing::info!("backbone reachable (no protocol ack yet)");
            }
        }

        // Heartbeat loop.
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        let mut nonce_seq: u64 = 0;
        loop {
            interval.tick().await;
            nonce_seq = nonce_seq.wrapping_add(1);
            if send_msg(&mut stream, &P2pMessage::Ping { nonce: nonce_seq })
                .await
                .is_err()
            {
                return Err(anyhow::anyhow!("backbone link closed"));
            }
        }
    }
}

async fn send_msg(stream: &mut TcpStream, msg: &P2pMessage) -> Result<()> {
    let payload = serde_json::to_vec(msg)?;
    let len = (payload.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&payload).await?;
    Ok(())
}

async fn recv_msg(stream: &mut TcpStream) -> Result<P2pMessage> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 1024 * 1024 {
        return Err(anyhow::anyhow!("message too large"));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(serde_json::from_slice(&buf)?)
}
