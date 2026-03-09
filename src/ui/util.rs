use gdk_pixbuf::prelude::*;
use gtk4::gdk;
use gtk4::glib;

use std::future::Future;
use std::path::PathBuf;

use super::state::{AppState, ImageCache};

pub fn spawn_async<T, Fut, F>(state: &AppState, task: Fut, callback: F)
where
    T: Send + 'static,
    Fut: Future<Output = T> + Send + 'static,
    F: FnOnce(T, &AppState) + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    let state = state.clone();

    state.rt.spawn(async move {
        let result = task.await;
        let _ = tx.send(result);
    });

    glib::spawn_future_local(async move {
        if let Ok(result) = rx.await {
            callback(result, &state);
        }
    });
}

fn disk_cache_dir() -> PathBuf {
    let base = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("plex-client").join("img")
}

fn url_to_cache_key(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    url.hash(&mut h);
    format!("{:016x}", h.finish())
}

struct DecodedImage {
    rgba: Vec<u8>,
    width: i32,
    height: i32,
}

fn decode_image_offthread(bytes: &[u8]) -> Option<DecodedImage> {
    let loader = gdk_pixbuf::PixbufLoader::new();
    loader.write(bytes).ok()?;
    loader.close().ok()?;
    let pixbuf = loader.pixbuf()?;

    let width = pixbuf.width();
    let height = pixbuf.height();
    let has_alpha = pixbuf.has_alpha();
    let rowstride = pixbuf.rowstride() as usize;
    let n_channels = if has_alpha { 4 } else { 3 };
    let src = unsafe { pixbuf.pixels() };

    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height as usize {
        let row_start = y * rowstride;
        for x in 0..width as usize {
            let offset = row_start + x * n_channels;
            rgba.push(src[offset]);
            rgba.push(src[offset + 1]);
            rgba.push(src[offset + 2]);
            if has_alpha {
                rgba.push(src[offset + 3]);
            } else {
                rgba.push(255);
            }
        }
    }

    Some(DecodedImage { rgba, width, height })
}

pub fn load_image_async(
    picture: &gtk4::Picture,
    url: &str,
    cache: ImageCache,
    http: reqwest::Client,
) {
    {
        let mut c = cache.borrow_mut();
        if let Some(texture) = c.get(url) {
            picture.set_paintable(Some(texture));
            return;
        }
    }

    let url_owned = url.to_string();
    let picture = picture.clone();
    let cache = cache.clone();

    let (tx, rx) = tokio::sync::oneshot::channel::<Option<DecodedImage>>();

    let url_fetch = url_owned.clone();
    tokio::spawn(async move {
        let cache_dir = disk_cache_dir();
        let cache_key = url_to_cache_key(&url_fetch);
        let cache_path = cache_dir.join(&cache_key);

        let bytes = if let Ok(b) = tokio::fs::read(&cache_path).await {
            Some(b)
        } else {
            let fetched = match http.get(&url_fetch).send().await {
                Ok(resp) if resp.status().is_success() => {
                    resp.bytes().await.ok().map(|b| b.to_vec())
                }
                Ok(resp) => {
                    eprintln!(
                        "plex-client: image HTTP {}: {}",
                        resp.status(),
                        &url_fetch[..url_fetch.find('?').unwrap_or(url_fetch.len())]
                    );
                    None
                }
                Err(e) => {
                    eprintln!("plex-client: image fetch error: {}", e);
                    None
                }
            };

            if let Some(ref bytes) = fetched {
                let _ = tokio::fs::create_dir_all(&cache_dir).await;
                let _ = tokio::fs::write(&cache_path, bytes).await;
            }

            fetched
        };

        let decoded = bytes.and_then(|b| decode_image_offthread(&b));
        let _ = tx.send(decoded);
    });

    glib::spawn_future_local(async move {
        if let Ok(Some(decoded)) = rx.await {
            let glib_bytes = glib::Bytes::from(&decoded.rgba);
            let texture = gdk::MemoryTexture::new(
                decoded.width,
                decoded.height,
                gdk::MemoryFormat::R8g8b8a8,
                &glib_bytes,
                (decoded.width * 4) as usize,
            );
            let texture = texture.upcast::<gdk::Texture>();
            picture.set_paintable(Some(&texture));
            cache.borrow_mut().insert(url_owned, texture);
        }
    });
}

pub fn format_duration(ms: i64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

pub fn format_time_secs(secs: f64) -> String {
    let total = secs as i64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}
