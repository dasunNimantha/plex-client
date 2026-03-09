use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub server_url: Option<String>,
    pub token: Option<String>,
    pub client_id: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: None,
            token: None,
            client_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("plex-client")
            .join("config.json")
    }

    pub fn is_configured(&self) -> bool {
        self.server_url.is_some() && self.token.is_some()
    }
}
