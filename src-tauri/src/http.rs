use reqwest::Client;

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
