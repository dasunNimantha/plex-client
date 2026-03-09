use anyhow::Result;
use std::process::{Child, Command};

pub struct MpvPlayer {
    process: Option<Child>,
    socket_path: String,
}

impl MpvPlayer {
    pub fn new() -> Self {
        Self {
            process: None,
            socket_path: format!("/tmp/plex-client-mpv-{}", std::process::id()),
        }
    }

    pub fn play(&mut self, url: &str, title: &str) -> Result<()> {
        self.stop();
        let child = Command::new("mpv")
            .arg(url)
            .arg(format!("--title={}", title))
            .arg(format!("--input-ipc-server={}", self.socket_path))
            .arg("--force-window=yes")
            .spawn()?;
        self.process = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.process = None;
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
