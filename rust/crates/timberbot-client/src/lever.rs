use reqwest::Client;
use timberbot_protocol::lever::{HttpAdapterState, HttpLeverState};
use timberbot_protocol::DEFAULT_LEVER_PORT;

use crate::error::{ClientError, Result};

/// Client for Timberborn's official 1.0+ built-in HTTP Lever / HTTP Adapter API
/// (default `http://127.0.0.1:8080/api/`).
#[derive(Debug, Clone)]
pub struct LeverClient {
    base_url: String,
    http: Client,
}

impl LeverClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let http = Client::builder().build()?;
        Ok(Self { base_url, http })
    }

    pub fn from_env() -> Result<Self> {
        let url = std::env::var("TIMBERBORN_LEVER_URL")
            .unwrap_or_else(|_| format!("http://127.0.0.1:{DEFAULT_LEVER_PORT}"));
        Self::new(url)
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn list_levers(&self) -> Result<Vec<HttpLeverState>> {
        let url = format!("{}/api/levers", self.base_url);
        let resp = self.http.get(&url).send().await?;
        parse_json_response(resp).await
    }

    pub async fn get_lever(&self, name: &str) -> Result<HttpLeverState> {
        let enc = urlencoding::encode(name);
        let url = format!("{}/api/levers/{enc}", self.base_url);
        let resp = self.http.get(&url).send().await?;
        parse_json_response(resp).await
    }

    pub async fn switch_on(&self, name: &str) -> Result<String> {
        let enc = urlencoding::encode(name);
        let url = format!("{}/api/switch-on/{enc}", self.base_url);
        let resp = self.http.post(&url).send().await?;
        parse_text_response(resp).await
    }

    pub async fn switch_off(&self, name: &str) -> Result<String> {
        let enc = urlencoding::encode(name);
        let url = format!("{}/api/switch-off/{enc}", self.base_url);
        let resp = self.http.post(&url).send().await?;
        parse_text_response(resp).await
    }

    pub async fn set_color(&self, name: &str, rrggbb: &str) -> Result<String> {
        let hex = rrggbb.trim().trim_start_matches('#');
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ClientError::InvalidArgument(format!(
                "color must be 6 hex characters (RRGGBB), got '{rrggbb}'"
            )));
        }
        let enc = urlencoding::encode(name);
        let url = format!("{}/api/color/{enc}/{hex}", self.base_url);
        let resp = self.http.post(&url).send().await?;
        parse_text_response(resp).await
    }

    pub async fn list_adapters(&self) -> Result<Vec<HttpAdapterState>> {
        let url = format!("{}/api/adapters", self.base_url);
        let resp = self.http.get(&url).send().await?;
        parse_json_response(resp).await
    }

    pub async fn get_adapter(&self, name: &str) -> Result<HttpAdapterState> {
        let enc = urlencoding::encode(name);
        let url = format!("{}/api/adapters/{enc}", self.base_url);
        let resp = self.http.get(&url).send().await?;
        parse_json_response(resp).await
    }
}

async fn parse_json_response<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(ClientError::ApiError {
            status: status.as_u16(),
            message: text,
        });
    }
    Ok(serde_json::from_str(&text)?)
}

async fn parse_text_response(resp: reqwest::Response) -> Result<String> {
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(ClientError::ApiError {
            status: status.as_u16(),
            message: text,
        });
    }
    Ok(text)
}
