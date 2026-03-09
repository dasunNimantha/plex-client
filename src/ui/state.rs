use gtk4 as gtk;
use gtk4::gdk;
use gtk4::glib;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::config::Config;
use crate::plex::{MediaItem, PlexClient};

use super::player_widget::PlayerWidget;

const IMAGE_CACHE_MAX: usize = 500;

/// LRU image cache: evicts oldest entries when capacity is exceeded.
pub struct LruImageCache {
    map: HashMap<String, gdk::Texture>,
    order: VecDeque<String>,
}

impl LruImageCache {
    fn new() -> Self {
        Self {
            map: HashMap::with_capacity(IMAGE_CACHE_MAX),
            order: VecDeque::with_capacity(IMAGE_CACHE_MAX),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<&gdk::Texture> {
        if self.map.contains_key(key) {
            self.order.retain(|k| k != key);
            self.order.push_back(key.to_string());
            self.map.get(key)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: String, texture: gdk::Texture) {
        if self.map.contains_key(&key) {
            self.order.retain(|k| k != &key);
        } else {
            while self.map.len() >= IMAGE_CACHE_MAX {
                if let Some(oldest) = self.order.pop_front() {
                    self.map.remove(&oldest);
                }
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, texture);
    }
}

pub type ImageCache = Rc<RefCell<LruImageCache>>;

#[derive(Clone)]
pub struct AppState {
    pub client: Rc<RefCell<Option<PlexClient>>>,
    pub player_widget: Rc<PlayerWidget>,
    pub image_cache: ImageCache,
    pub current_items: Rc<RefCell<Vec<MediaItem>>>,
    pub rt: tokio::runtime::Handle,
    pub main_stack: gtk::Stack,
    pub config: Rc<RefCell<Config>>,
    pub progress_timer: Rc<RefCell<Option<glib::SourceId>>>,
    pub sidebar_list: gtk::ListBox,
}

impl AppState {
    pub fn new(
        rt: tokio::runtime::Handle,
        player_widget: Rc<PlayerWidget>,
        main_stack: gtk::Stack,
        config: Config,
        sidebar_list: gtk::ListBox,
    ) -> Self {
        Self {
            client: Rc::new(RefCell::new(None)),
            player_widget,
            image_cache: Rc::new(RefCell::new(LruImageCache::new())),
            current_items: Rc::new(RefCell::new(Vec::new())),
            rt,
            main_stack,
            config: Rc::new(RefCell::new(config)),
            progress_timer: Rc::new(RefCell::new(None)),
            sidebar_list,
        }
    }
}
