use crate::lockfile::read_lockfile;
use base64::Engine;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async_tls_with_config, Connector};

/// Watch the local client websocket and wake the main loop whenever our own
/// presence changes, so state transitions are picked up at once instead of on
/// the next poll. The poll stays as a fallback, so a websocket failure or a
/// closed client is harmless.
pub async fn run_presence_socket(notify: Arc<Notify>) {
    loop {
        let _ = listen_once(&notify).await;
        // The lockfile is absent while the client is closed; retry after a
        // short pause rather than busy looping.
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn listen_once(notify: &Arc<Notify>) -> Result<(), Box<dyn std::error::Error>> {
    let lf = read_lockfile().map_err(|_| "no lockfile")?;
    let mut request = format!("wss://127.0.0.1:{}/", lf.port).into_client_request()?;
    let token = base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", lf.password));
    request
        .headers_mut()
        .insert("Authorization", HeaderValue::from_str(&format!("Basic {token}"))?);

    // The local client serves this over a self signed certificate.
    let tls = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()?;
    let (mut socket, _) =
        connect_async_tls_with_config(request, None, false, Some(Connector::NativeTls(tls))).await?;

    // Opcode 5 subscribes to an event in the client's websocket protocol.
    socket
        .send(Message::Text(
            "[5, \"OnJsonApiEvent_chat_v4_presences\"]".into(),
        ))
        .await?;

    // Any presence event is enough to wake the loop; it re-reads the local
    // presence itself. Notify coalesces bursts into a single wake.
    while let Some(msg) = socket.next().await {
        match msg? {
            Message::Text(_) | Message::Binary(_) => notify.notify_one(),
            Message::Close(_) => break,
            _ => {}
        }
    }
    Ok(())
}
