use anyhow::Result;
use serde::Deserialize;
use std::sync::Arc;

const PLEX_PRODUCT: &str = "Plex Client for Linux";
const PLEX_VERSION: &str = env!("CARGO_PKG_VERSION");
const PLEX_PLATFORM: &str = "Linux";

#[derive(Clone)]
pub struct PlexClient {
    pub http: reqwest::Client,
    server_url: Arc<str>,
    token: Arc<str>,
    client_id: Arc<str>,
}

impl PlexClient {
    pub async fn connect(server_url: &str, token: &str, client_id: &str) -> Result<Self> {
        let server_url: Arc<str> = server_url.trim_end_matches('/').into();
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .pool_max_idle_per_host(8)
            .build()?;

        let resp = http
            .get(&*server_url)
            .headers(Self::plex_headers(token, client_id))
            .header("Accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Connection failed: HTTP {}", resp.status());
        }

        Ok(Self {
            http,
            server_url,
            token: token.into(),
            client_id: client_id.into(),
        })
    }

    fn plex_headers(token: &str, client_id: &str) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert("X-Plex-Token", token.parse().unwrap());
        h.insert("X-Plex-Client-Identifier", client_id.parse().unwrap());
        h.insert("X-Plex-Product", PLEX_PRODUCT.parse().unwrap());
        h.insert("X-Plex-Version", PLEX_VERSION.parse().unwrap());
        h.insert("X-Plex-Platform", PLEX_PLATFORM.parse().unwrap());
        h
    }

    async fn get_json(&self, path: &str) -> Result<String> {
        let url = format!("{}{}", self.server_url, path);
        Ok(self
            .http
            .get(&url)
            .headers(Self::plex_headers(&self.token, &self.client_id))
            .header("Accept", "application/json")
            .send()
            .await?
            .text()
            .await?)
    }

    pub async fn get_libraries(&self) -> Result<Vec<Library>> {
        let text = self.get_json("/library/sections").await?;
        let resp: PlexResponse<LibraryContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.directory.unwrap_or_default())
    }

    pub async fn get_library_items(&self, section_id: &str) -> Result<Vec<MediaItem>> {
        let text = self
            .get_json(&format!("/library/sections/{}/all", section_id))
            .await?;
        let resp: PlexResponse<MetadataContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.metadata.unwrap_or_default())
    }

    pub async fn get_children(&self, rating_key: &str) -> Result<Vec<MediaItem>> {
        let text = self
            .get_json(&format!("/library/metadata/{}/children", rating_key))
            .await?;
        let resp: PlexResponse<MetadataContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.metadata.unwrap_or_default())
    }

    pub async fn get_on_deck(&self) -> Result<Vec<MediaItem>> {
        let text = self.get_json("/library/onDeck").await?;
        let resp: PlexResponse<MetadataContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.metadata.unwrap_or_default())
    }

    pub async fn get_recently_added(&self) -> Result<Vec<MediaItem>> {
        let text = self.get_json("/library/recentlyAdded").await?;
        let resp: PlexResponse<MetadataContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.metadata.unwrap_or_default())
    }

    pub async fn search(&self, query: &str) -> Result<Vec<MediaItem>> {
        let encoded = urlencoding::encode(query);
        let text = self
            .get_json(&format!("/search?query={}", encoded))
            .await?;
        let resp: PlexResponse<MetadataContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.metadata.unwrap_or_default())
    }

    pub async fn report_progress(
        &self,
        rating_key: &str,
        offset_ms: i64,
        state: &str,
        duration_ms: i64,
    ) -> Result<()> {
        let url = format!(
            "{}/:/timeline?ratingKey={}&key=%2Flibrary%2Fmetadata%2F{}&state={}&time={}&duration={}&X-Plex-Token={}",
            self.server_url, rating_key, rating_key, state, offset_ms, duration_ms, self.token
        );
        let _ = self
            .http
            .get(&url)
            .headers(Self::plex_headers(&self.token, &self.client_id))
            .send()
            .await;
        Ok(())
    }

    pub fn stream_url(&self, part_key: &str) -> String {
        format!(
            "{}{}?X-Plex-Token={}",
            self.server_url, part_key, self.token
        )
    }

    pub fn poster_url(&self, thumb: &str) -> String {
        let encoded_url = urlencoding::encode(thumb);
        format!(
            "{}/photo/:/transcode?url={}&width=300&height=450&minSize=1&X-Plex-Token={}",
            self.server_url, encoded_url, self.token
        )
    }

    pub fn poster_url_full(&self, thumb: &str) -> String {
        format!(
            "{}{}?X-Plex-Token={}",
            self.server_url, thumb, self.token
        )
    }

}

// --- Plex API response models ---

#[derive(Deserialize)]
pub struct PlexResponse<T> {
    #[serde(rename = "MediaContainer")]
    pub media_container: T,
}

#[derive(Deserialize)]
pub struct LibraryContainer {
    #[serde(rename = "Directory")]
    pub directory: Option<Vec<Library>>,
}

#[derive(Deserialize)]
pub struct MetadataContainer {
    #[serde(rename = "Metadata")]
    pub metadata: Option<Vec<MediaItem>>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Library {
    pub key: String,
    pub title: String,
    #[serde(rename = "type")]
    pub lib_type: String,
}

#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct MediaItem {
    #[serde(rename = "ratingKey")]
    pub rating_key: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub summary: Option<String>,
    pub year: Option<i32>,
    pub thumb: Option<String>,
    pub art: Option<String>,
    pub duration: Option<i64>,
    pub rating: Option<f64>,
    #[serde(rename = "audienceRating")]
    pub audience_rating: Option<f64>,
    #[serde(rename = "contentRating")]
    pub content_rating: Option<String>,
    pub index: Option<i32>,
    #[serde(rename = "parentIndex")]
    pub parent_index: Option<i32>,
    #[serde(rename = "parentTitle")]
    pub parent_title: Option<String>,
    #[serde(rename = "grandparentTitle")]
    pub grandparent_title: Option<String>,
    #[serde(rename = "viewOffset")]
    pub view_offset: Option<i64>,
    #[serde(rename = "leafCount")]
    pub leaf_count: Option<i32>,
    #[serde(rename = "viewedLeafCount")]
    pub viewed_leaf_count: Option<i32>,
    #[serde(rename = "Media")]
    pub media: Option<Vec<Media>>,
}

#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct Media {
    pub duration: Option<i64>,
    pub bitrate: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    #[serde(rename = "videoCodec")]
    pub video_codec: Option<String>,
    #[serde(rename = "audioCodec")]
    pub audio_codec: Option<String>,
    #[serde(rename = "videoResolution")]
    pub video_resolution: Option<String>,
    pub container: Option<String>,
    #[serde(rename = "Part")]
    pub parts: Option<Vec<Part>>,
}

#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct Part {
    pub key: Option<String>,
    pub file: Option<String>,
    pub size: Option<i64>,
    pub container: Option<String>,
}

impl MediaItem {
    pub fn display_title(&self) -> String {
        self.title.clone().unwrap_or_else(|| "Unknown".into())
    }

    pub fn stream_part_key(&self) -> Option<&str> {
        self.media
            .as_ref()?
            .first()?
            .parts
            .as_ref()?
            .first()?
            .key
            .as_deref()
    }

    pub fn media_info_string(&self) -> String {
        let mut parts = Vec::new();
        if let Some(media) = self.media.as_ref().and_then(|m| m.first()) {
            if let Some(res) = &media.video_resolution {
                parts.push(format!("{}p", res));
            }
            if let Some(vc) = &media.video_codec {
                parts.push(vc.to_uppercase());
            }
            if let Some(ac) = &media.audio_codec {
                parts.push(ac.to_uppercase());
            }
            if let Some(c) = &media.container {
                parts.push(c.to_uppercase());
            }
        }
        parts.join(" · ")
    }
}
