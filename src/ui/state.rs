use gtk4 as gtk;
use gtk4::gdk;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::config::Config;
use crate::plex::{MediaItem, PlexClient};

use super::player_widget::PlayerWidget;

pub type ImageCache = Rc<RefCell<HashMap<String, gdk::Texture>>>;

#[derive(Clone)]
pub struct AppState {
    pub client: Rc<RefCell<Option<PlexClient>>>,
    pub player_widget: Rc<PlayerWidget>,
    pub image_cache: ImageCache,
    pub current_items: Rc<RefCell<Vec<MediaItem>>>,
    pub rt: tokio::runtime::Handle,
    pub main_stack: gtk::Stack,
    pub config: Rc<RefCell<Config>>,
}

impl AppState {
    pub fn new(
        rt: tokio::runtime::Handle,
        player_widget: Rc<PlayerWidget>,
        main_stack: gtk::Stack,
        config: Config,
    ) -> Self {
        Self {
            client: Rc::new(RefCell::new(None)),
            player_widget,
            image_cache: Rc::new(RefCell::new(HashMap::new())),
            current_items: Rc::new(RefCell::new(Vec::new())),
            rt,
            main_stack,
            config: Rc::new(RefCell::new(config)),
        }
    }
}
