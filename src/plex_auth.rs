use anyhow::Result;
use serde::Deserialize;

const PLEX_PRODUCT: &str = "Plex Client for Linux";
const PLEX_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize, Debug)]
pub struct PinResponse {
    pub id: i64,
    pub code: String,
    #[serde(rename = "authToken")]
    pub auth_token: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlexResource {
    pub name: String,
    pub provides: String,
    #[serde(rename = "publicAddress")]
    pub public_address: Option<String>,
    #[serde(rename = "accessToken")]
    pub access_token: Option<String>,
    pub connections: Option<Vec<PlexConnection>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlexConnection {
    pub uri: String,
    pub local: Option<bool>,
    pub protocol: Option<String>,
    pub address: Option<String>,
    pub port: Option<u16>,
}

impl PlexResource {
    /// Returns connection URIs sorted by preference:
    /// 1. Local HTTP (direct private IP, no TLS issues)
    /// 2. Local HTTPS (plex.direct, may need special certs)
    /// 3. Remote HTTP
    /// 4. Remote HTTPS
    pub fn ranked_connection_uris(&self) -> Vec<String> {
        let Some(conns) = &self.connections else {
            return Vec::new();
        };

        let mut ranked: Vec<(i32, &PlexConnection)> = conns
            .iter()
            .map(|c| {
                let is_local = c.local == Some(true);
                let is_http = c.uri.starts_with("http://");
                let is_plex_direct = c.uri.contains("plex.direct");
                let score = match (is_local, is_http, is_plex_direct) {
                    (true, true, _) => 0,    // local + http = best
                    (true, false, false) => 1, // local + https (direct IP)
                    (true, false, true) => 2,  // local + plex.direct https
                    (false, true, _) => 3,     // remote + http
                    (false, false, false) => 4, // remote + https (direct IP)
                    (false, false, true) => 5,  // remote + plex.direct https
                };
                (score, c)
            })
            .collect();

        ranked.sort_by_key(|(score, _)| *score);
        ranked.into_iter().map(|(_, c)| c.uri.clone()).collect()
    }
}

pub async fn request_pin(client_id: &str) -> Result<PinResponse> {
    let http = reqwest::Client::new();
    let resp = http
        .post("https://plex.tv/api/v2/pins")
        .header("Accept", "application/json")
        .form(&[
            ("strong", "true"),
            ("X-Plex-Product", PLEX_PRODUCT),
            ("X-Plex-Version", PLEX_VERSION),
            ("X-Plex-Client-Identifier", client_id),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Failed to request PIN: HTTP {}", resp.status());
    }

    Ok(resp.json().await?)
}

pub fn auth_url(client_id: &str, code: &str) -> String {
    format!(
        "https://app.plex.tv/auth#?clientID={}&code={}&context%5Bdevice%5D%5Bproduct%5D={}",
        urlencoding::encode(client_id),
        urlencoding::encode(code),
        urlencoding::encode(PLEX_PRODUCT),
    )
}

pub async fn check_pin(client_id: &str, pin_id: i64, code: &str) -> Result<Option<String>> {
    let http = reqwest::Client::new();
    let url = format!("https://plex.tv/api/v2/pins/{}", pin_id);
    let resp = http
        .get(&url)
        .header("Accept", "application/json")
        .query(&[
            ("code", code),
            ("X-Plex-Client-Identifier", client_id),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Failed to check PIN: HTTP {}", resp.status());
    }

    let pin: PinResponse = resp.json().await?;
    Ok(pin.auth_token)
}

pub async fn get_servers(token: &str, client_id: &str) -> Result<Vec<PlexResource>> {
    let http = reqwest::Client::new();
    let resp = http
        .get("https://plex.tv/api/v2/resources")
        .header("Accept", "application/json")
        .header("X-Plex-Token", token)
        .header("X-Plex-Client-Identifier", client_id)
        .query(&[("includeHttps", "1"), ("includeRelay", "1")])
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Failed to get servers: HTTP {}", resp.status());
    }

    let resources: Vec<PlexResource> = resp.json().await?;
    Ok(resources
        .into_iter()
        .filter(|r| r.provides.contains("server"))
        .collect())
}

/// Tries each connection URI for a server in preference order,
/// returning the first one that successfully connects.
pub async fn find_working_connection(
    server: &PlexResource,
    token: &str,
    client_id: &str,
) -> Result<String> {
    let uris = server.ranked_connection_uris();
    if uris.is_empty() {
        anyhow::bail!("No connection URIs available for server '{}'", server.name);
    }

    // Build a client that accepts self-signed/plex.direct certs for probing
    let probe_http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .danger_accept_invalid_certs(true)
        .build()?;

    for uri in &uris {
        let result = probe_http
            .get(uri)
            .header("X-Plex-Token", token)
            .header("X-Plex-Client-Identifier", client_id)
            .header("Accept", "application/json")
            .send()
            .await;

        if let Ok(resp) = result {
            if resp.status().is_success() {
                return Ok(uri.clone());
            }
        }
    }

    anyhow::bail!(
        "Could not reach server '{}'. Tried {} connection(s): {}",
        server.name,
        uris.len(),
        uris.join(", ")
    )
}
