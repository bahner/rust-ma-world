use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info};

use crate::status::SharedWorldStatus;

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
