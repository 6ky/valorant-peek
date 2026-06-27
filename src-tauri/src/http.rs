use reqwest::header::HeaderMap;
use reqwest::{Client, RequestBuilder};
use serde_json::Value;
use std::time::Duration;

/// Client for the local Riot client API on 127.0.0.1, which serves a
/// self-signed certificate. Accepting invalid certs is scoped to localhost only.
pub fn local_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("failed to build local http client")
}

/// Client for the public Riot PVP endpoints and valorant-api.com.
pub fn pvp_client() -> Client {
    Client::builder()
        .build()
        .expect("failed to build pvp http client")
}

/// Send a request with up to three tries, backing off on a rate limit. The
/// builder closure is called fresh for each try. Returns None on any non-success
/// status (so callers keep their last known value instead of caching an error
/// body) or once the tries run out. This is the single retry path every Riot
/// request goes through.
async fn send_retry<F: Fn() -> RequestBuilder>(make: F) -> Option<Value> {
    for _ in 0..3 {
        let resp = make().send().await.ok()?;
        if resp.status().as_u16() == 429 {
            let secs = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(5);
            tokio::time::sleep(Duration::from_secs(secs + 1)).await;
            continue;
        }
        if !resp.status().is_success() {
            return None;
        }
        return resp.json().await.ok();
    }
    None
}

/// Authed GET returning parsed JSON, with retries.
pub async fn get_json_retry(url: &str, headers: HeaderMap) -> Option<Value> {
    send_retry(|| pvp_client().get(url).headers(headers.clone())).await
}

/// Authed PUT with a JSON body returning parsed JSON, with retries. Used for the
/// batch name-service call.
pub async fn put_json_retry(url: &str, headers: HeaderMap, body: &Value) -> Option<Value> {
    send_retry(|| pvp_client().put(url).headers(headers.clone()).json(body)).await
}
