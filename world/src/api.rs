use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize)]
pub struct WorldStatus {
    pub slug: String,
    pub owner: String,
    pub config_path: String,
    pub status_api_bind: String,
    pub ipns_endpoint: Option<String>,
    pub started_at_unix: u64,
    pub endpoint_id: Option<String>,
    pub services: Vec<String>,
    pub inbox_messages: u64,
    pub ipfs_messages: u64,
    pub last_inbox_at_unix: Option<u64>,
    pub last_ipfs_at_unix: Option<u64>,
}

pub type SharedWorldStatus = Arc<RwLock<WorldStatus>>;

pub fn new_shared_status(
    slug: String,
    owner: String,
    config_path: String,
    status_api_bind: String,
    ipns_endpoint: Option<String>,
    started_at_unix: u64,
) -> SharedWorldStatus {
    Arc::new(RwLock::new(WorldStatus {
        slug,
        owner,
        config_path,
        status_api_bind,
        ipns_endpoint,
        started_at_unix,
        endpoint_id: None,
        services: Vec::new(),
        inbox_messages: 0,
        ipfs_messages: 0,
        last_inbox_at_unix: None,
        last_ipfs_at_unix: None,
    }))
}

pub async fn set_endpoint_metadata(
    status: &SharedWorldStatus,
    endpoint_id: String,
    services: Vec<String>,
) {
    let mut state = status.write().await;
    state.endpoint_id = Some(endpoint_id);
    state.services = services;
}

pub async fn mark_inbox(status: &SharedWorldStatus, now_unix: u64) {
    let mut state = status.write().await;
    state.inbox_messages += 1;
    state.last_inbox_at_unix = Some(now_unix);
}

pub async fn mark_ipfs(status: &SharedWorldStatus, now_unix: u64) {
    let mut state = status.write().await;
    state.ipfs_messages += 1;
    state.last_ipfs_at_unix = Some(now_unix);
}

pub async fn run_status_api(bind: String, status: SharedWorldStatus) -> Result<()> {
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("bind status api at {bind}"))?;
    let local_addr = listener.local_addr().ok();
    if let Some(addr) = local_addr {
        info!(addr = %addr, "status api listening");
    } else {
        info!(bind = %bind, "status api listening");
    }

    loop {
        let (mut stream, peer) = listener.accept().await?;
        let status = Arc::clone(&status);
        tokio::spawn(async move {
            if let Err(err) = handle_status_connection(&mut stream, peer, status).await {
                debug!(error = %err, "status api request handling failed");
            }
        });
    }
}

async fn handle_status_connection(
    stream: &mut TcpStream,
    peer: SocketAddr,
    status: SharedWorldStatus,
) -> Result<()> {
    let mut buf = [0u8; 4096];
    let read_len = stream.read(&mut buf).await?;
    if read_len == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buf[..read_len]);
    let request_line = request.lines().next().unwrap_or_default();
    info!(peer = %peer, request_line = %request_line, "status api incoming request");
    debug!(peer = %peer, request_line = %request_line, "status api request");

    if request_line.starts_with("GET /status.json ") || request_line.starts_with("GET /status.json?") {
        let snapshot = { status.read().await.clone() };
        let body = serde_json::to_vec(&snapshot)?;
        write_http_response(stream, 200, "OK", "application/json", &body).await?;
        return Ok(());
    }

    write_http_response(
        stream,
        404,
        "Not Found",
        "text/plain; charset=utf-8",
        b"not found\n",
    )
    .await?;
    Ok(())
}

async fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.shutdown().await?;
    Ok(())
}
