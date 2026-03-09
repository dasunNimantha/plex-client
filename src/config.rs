use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum HwdecMode {
    #[serde(rename = "vaapi-copy")]
    VaapiCopy,
    #[serde(rename = "nvdec-copy")]
    NvdecCopy,
    #[serde(rename = "vdpau-copy")]
    VdpauCopy,
    #[serde(rename = "auto-copy")]
    AutoCopy,
    #[serde(rename = "no")]
    None,
}

impl HwdecMode {
    pub fn as_mpv_value(&self) -> &str {
        match self {
            Self::VaapiCopy => "vaapi-copy",
            Self::NvdecCopy => "nvdec-copy",
            Self::VdpauCopy => "vdpau-copy",
            Self::AutoCopy => "auto-copy",
            Self::None => "no",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::VaapiCopy => "VA-API (Intel / AMD)",
            Self::NvdecCopy => "NVDEC (NVIDIA)",
            Self::VdpauCopy => "VDPAU (NVIDIA legacy)",
            Self::AutoCopy => "Auto",
            Self::None => "Software (CPU)",
        }
    }
}

#[derive(Clone)]
pub struct DetectedHwdec {
    pub mode: HwdecMode,
    pub label: String,
}

fn any_exists(paths: &[&str]) -> bool {
    paths.iter().any(|p| std::path::Path::new(p).exists())
}

pub fn detect_available_hwdec() -> Vec<DetectedHwdec> {
    let mut available = Vec::new();

    let has_render_node = any_exists(&["/dev/dri/renderD128", "/dev/dri/renderD129"]);

    let vaapi_driver_dirs = [
        "/usr/lib/x86_64-linux-gnu/dri",
        "/usr/lib64/dri",
        "/usr/lib/dri",
    ];
    let vaapi_drivers = ["iHD_drv_video.so", "i965_drv_video.so", "radeonsi_drv_video.so"];
    let has_vaapi = has_render_node
        && vaapi_driver_dirs.iter().any(|dir| {
            vaapi_drivers
                .iter()
                .any(|drv| std::path::Path::new(dir).join(drv).exists())
        });

    let has_nvidia = std::path::Path::new("/dev/nvidia0").exists();

    let vdpau_dirs = [
        "/usr/lib/x86_64-linux-gnu/vdpau",
        "/usr/lib64/vdpau",
        "/usr/lib/vdpau",
    ];
    let has_vdpau = vdpau_dirs.iter().any(|dir| {
        std::path::Path::new(dir)
            .join("libvdpau_nvidia.so")
            .exists()
    });

    if has_vaapi {
        available.push(DetectedHwdec {
            mode: HwdecMode::VaapiCopy,
            label: "VA-API (Intel / AMD)".into(),
        });
    }

    if has_nvidia {
        available.push(DetectedHwdec {
            mode: HwdecMode::NvdecCopy,
            label: "NVDEC (NVIDIA)".into(),
        });
    }

    if has_vdpau {
        available.push(DetectedHwdec {
            mode: HwdecMode::VdpauCopy,
            label: "VDPAU (NVIDIA legacy)".into(),
        });
    }

    available.push(DetectedHwdec {
        mode: HwdecMode::AutoCopy,
        label: "Auto (let mpv decide)".into(),
    });

    available.push(DetectedHwdec {
        mode: HwdecMode::None,
        label: "Software (CPU only)".into(),
    });

    available
}

pub fn best_default_hwdec() -> HwdecMode {
    let available = detect_available_hwdec();
    for entry in &available {
        match entry.mode {
            HwdecMode::VaapiCopy => return HwdecMode::VaapiCopy,
            HwdecMode::NvdecCopy => return HwdecMode::NvdecCopy,
            _ => {}
        }
    }
    HwdecMode::AutoCopy
}

fn default_seek_seconds() -> u32 {
    10
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub server_url: Option<String>,
    pub token: Option<String>,
    pub client_id: String,
    #[serde(default = "best_default_hwdec")]
    pub hwdec: HwdecMode,
    #[serde(default = "default_seek_seconds")]
    pub seek_seconds: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: None,
            token: None,
            client_id: uuid::Uuid::new_v4().to_string(),
            hwdec: best_default_hwdec(),
            seek_seconds: 10,
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
