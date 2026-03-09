use anyhow::Result;
use serde::Deserialize;

#[derive(Clone)]
pub struct PlexClient {
    http: reqwest::blocking::Client,
    pub server_url: String,
    pub token: String,
}

impl PlexClient {
    pub fn connect(server_url: &str, token: &str) -> Result<Self> {
        let server_url = server_url.trim_end_matches('/').to_string();
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let resp = http
            .get(&server_url)
            .header("X-Plex-Token", token)
            .header("Accept", "application/json")
            .send()?;

        if !resp.status().is_success() {
            anyhow::bail!("Connection failed: HTTP {}", resp.status());
        }

        Ok(Self {
            http,
            server_url,
            token: token.to_string(),
        })
    }

    fn get_json(&self, path: &str) -> Result<String> {
        let url = format!("{}{}", self.server_url, path);
        Ok(self
            .http
            .get(&url)
            .header("X-Plex-Token", &self.token)
            .header("Accept", "application/json")
            .send()?
            .text()?)
    }

    pub fn get_libraries(&self) -> Result<Vec<Library>> {
        let text = self.get_json("/library/sections")?;
        let resp: PlexResponse<LibraryContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.directory.unwrap_or_default())
    }

    pub fn get_library_items(&self, section_id: &str) -> Result<Vec<MediaItem>> {
        let text = self.get_json(&format!("/library/sections/{}/all", section_id))?;
        let resp: PlexResponse<MetadataContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.metadata.unwrap_or_default())
    }

    pub fn get_children(&self, rating_key: &str) -> Result<Vec<MediaItem>> {
        let text = self.get_json(&format!("/library/metadata/{}/children", rating_key))?;
        let resp: PlexResponse<MetadataContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.metadata.unwrap_or_default())
    }

    pub fn get_on_deck(&self) -> Result<Vec<MediaItem>> {
        let text = self.get_json("/library/onDeck")?;
        let resp: PlexResponse<MetadataContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.metadata.unwrap_or_default())
    }

    pub fn get_recently_added(&self) -> Result<Vec<MediaItem>> {
        let text = self.get_json("/library/recentlyAdded")?;
        let resp: PlexResponse<MetadataContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.metadata.unwrap_or_default())
    }

    pub fn search(&self, query: &str) -> Result<Vec<MediaItem>> {
        let encoded = urlencoding::encode(query);
        let text = self.get_json(&format!("/search?query={}", encoded))?;
        let resp: PlexResponse<MetadataContainer> = serde_json::from_str(&text)?;
        Ok(resp.media_container.metadata.unwrap_or_default())
    }

    pub fn stream_url(&self, part_key: &str) -> String {
        format!(
            "{}{}?X-Plex-Token={}",
            self.server_url, part_key, self.token
        )
    }

    pub fn poster_url(&self, thumb: &str) -> String {
        format!(
            "{}{}?X-Plex-Token={}",
            self.server_url, thumb, self.token
        )
    }

    pub fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        Ok(self.http.get(url).send()?.bytes()?.to_vec())
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
