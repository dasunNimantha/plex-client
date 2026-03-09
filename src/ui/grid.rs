use gtk4 as gtk;
use gtk4::pango;
use libadwaita as adw;

use gtk::prelude::*;

use crate::plex::{Hub, MediaItem, PlexClient};

use super::detail;
use super::state::AppState;
use super::util;

pub fn populate_grid(
    flow_box: &gtk::FlowBox,
    items: &[MediaItem],
    state: &AppState,
    nav_view: &adw::NavigationView,
    toast_overlay: &adw::ToastOverlay,
    window: &adw::ApplicationWindow,
) {
    while let Some(child) = flow_box.first_child() {
        flow_box.remove(&child);
    }

    *state.current_items.borrow_mut() = items.to_vec();

    let plex = {
        let c = state.client.borrow();
        match c.as_ref() {
            Some(p) => p.clone(),
            None => return,
        }
    };

    let cards: Vec<gtk::Button> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let poster_url = item.thumb.as_ref().map(|t| plex.poster_url(t));
            let card = create_poster_card(item, poster_url, &state.image_cache, &plex.http);

            let card_btn = gtk::Button::builder()
                .child(&card)
                .css_classes(["flat"])
                .build();

            let state = state.clone();
            let nav_view = nav_view.clone();
            let toast_overlay = toast_overlay.clone();
            let window = window.clone();

            card_btn.connect_clicked(move |_| {
                let items = state.current_items.borrow();
                let Some(item) = items.get(idx) else { return };
                handle_item_click(item, &nav_view, &state, &toast_overlay, &window);
            });

            card_btn
        })
        .collect();

    for card_btn in cards {
        flow_box.insert(&card_btn, -1);
    }
}

fn is_landscape_hub(hub: &Hub) -> bool {
    hub.hub_identifier
        .as_deref()
        .map(|id| id.contains("continue") || id.contains("recentlyViewed"))
        .unwrap_or(false)
}

pub fn build_home_content(
    hubs: &[Hub],
    state: &AppState,
    nav_view: &adw::NavigationView,
    toast_overlay: &adw::ToastOverlay,
    window: &adw::ApplicationWindow,
) -> gtk::ScrolledWindow {
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    vbox.set_margin_bottom(40);
    vbox.set_margin_top(8);

    let plex = {
        let c = state.client.borrow();
        match c.as_ref() {
            Some(p) => p.clone(),
            None => {
                scroll.set_child(Some(&vbox));
                return scroll;
            }
        }
    };

    let mut first_hub = true;

    for hub in hubs {
        let items = hub.metadata.as_deref().unwrap_or_default();
        if items.is_empty() {
            continue;
        }

        if !first_hub {
            let divider = gtk::Box::builder()
                .css_classes(["hub-divider"])
                .build();
            vbox.append(&divider);
        }
        first_hub = false;

        let raw_title = hub.title.as_deref().unwrap_or("Untitled");
        let upper_title = raw_title.to_uppercase();

        let title_label = gtk::Label::builder()
            .label(&upper_title)
            .halign(gtk::Align::Start)
            .css_classes(["hub-title"])
            .build();
        vbox.append(&title_label);

        let landscape = is_landscape_hub(hub);

        let shelf_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .build();
        shelf_scroll.set_margin_start(20);
        shelf_scroll.set_margin_end(20);
        shelf_scroll.add_css_class("hub-section");

        let shelf_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        shelf_box.set_margin_end(16);

        for item in items {
            let card = if landscape {
                create_landscape_card(item, &plex, &state.image_cache, &plex.http)
            } else {
                let poster_url = item.thumb.as_ref().map(|t| plex.poster_url(t));
                create_poster_card(item, poster_url, &state.image_cache, &plex.http)
            };

            let card_btn = gtk::Button::builder()
                .child(&card)
                .css_classes(["flat"])
                .build();

            let item = item.clone();
            let nav_view = nav_view.clone();
            let state = state.clone();
            let toast_overlay = toast_overlay.clone();
            let window = window.clone();

            card_btn.connect_clicked(move |_| {
                handle_item_click(&item, &nav_view, &state, &toast_overlay, &window);
            });

            shelf_box.append(&card_btn);
        }

        shelf_scroll.set_child(Some(&shelf_box));
        vbox.append(&shelf_scroll);
    }

    scroll.set_child(Some(&vbox));
    scroll
}

fn create_landscape_card(
    item: &MediaItem,
    plex: &PlexClient,
    image_cache: &super::state::ImageCache,
    http: &reqwest::Client,
) -> gtk::Box {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .css_classes(["landscape-card"])
        .build();

    let picture = gtk::Picture::builder()
        .width_request(240)
        .height_request(135)
        .content_fit(gtk::ContentFit::Cover)
        .css_classes(["landscape-image"])
        .build();

    let image_url = item
        .art
        .as_ref()
        .map(|a| plex.art_url(a, 400, 225))
        .or_else(|| item.thumb.as_ref().map(|t| plex.poster_url(t)));

    if let Some(url) = image_url {
        util::load_image_async(&picture, &url, image_cache.clone(), http.clone());
    }

    card.append(&picture);

    if let (Some(offset), Some(duration)) = (item.view_offset, item.duration) {
        if duration > 0 && offset > 0 {
            let fraction = (offset as f64 / duration as f64).clamp(0.0, 1.0);
            let track = gtk::Box::builder()
                .css_classes(["poster-progress-track"])
                .build();
            let fill = gtk::Box::builder()
                .css_classes(["poster-progress-fill"])
                .hexpand(false)
                .build();
            fill.set_size_request((240.0 * fraction) as i32, -1);
            track.append(&fill);
            card.append(&track);
        }
    }

    let info_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    info_box.set_margin_top(4);

    let text_col = gtk::Box::new(gtk::Orientation::Vertical, 0);
    text_col.set_hexpand(true);

    let title = gtk::Label::new(Some(&item.display_title()));
    title.set_ellipsize(pango::EllipsizeMode::End);
    title.set_max_width_chars(22);
    title.add_css_class("landscape-title");
    title.set_halign(gtk::Align::Start);
    text_col.append(&title);

    let sub = match item.item_type.as_deref() {
        Some("episode") => {
            let show = item.grandparent_title.as_deref().unwrap_or("");
            let s = item.parent_index.unwrap_or(0);
            let e = item.index.unwrap_or(0);
            format!("{} \u{00b7} S{:02}E{:02}", show, s, e)
        }
        _ => item.year.map(|y| y.to_string()).unwrap_or_default(),
    };
    if !sub.is_empty() {
        let sub_label = gtk::Label::new(Some(&sub));
        sub_label.set_ellipsize(pango::EllipsizeMode::End);
        sub_label.set_max_width_chars(22);
        sub_label.add_css_class("landscape-info");
        sub_label.set_halign(gtk::Align::Start);
        text_col.append(&sub_label);
    }

    info_box.append(&text_col);

    if let (Some(offset), Some(duration)) = (item.view_offset, item.duration) {
        if duration > 0 && offset > 0 {
            let remaining_min = (duration - offset) / 60000;
            let remaining_text = format!("{} min left", remaining_min);
            let remaining_label = gtk::Label::builder()
                .label(&remaining_text)
                .css_classes(["landscape-remaining"])
                .halign(gtk::Align::End)
                .valign(gtk::Align::Start)
                .build();
            info_box.append(&remaining_label);
        }
    }

    card.append(&info_box);
    card
}

fn create_poster_card(
    item: &MediaItem,
    poster_url: Option<String>,
    image_cache: &super::state::ImageCache,
    http: &reqwest::Client,
) -> gtk::Box {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .css_classes(["poster-card"])
        .halign(gtk::Align::Center)
        .build();

    let picture = gtk::Picture::builder()
        .width_request(130)
        .height_request(195)
        .content_fit(gtk::ContentFit::Cover)
        .css_classes(["poster-image"])
        .build();

    if let Some(url) = poster_url {
        util::load_image_async(&picture, &url, image_cache.clone(), http.clone());
    }

    card.append(&picture);

    if let (Some(offset), Some(duration)) = (item.view_offset, item.duration) {
        if duration > 0 && offset > 0 {
            let fraction = (offset as f64 / duration as f64).clamp(0.0, 1.0);
            let track = gtk::Box::builder()
                .css_classes(["poster-progress-track"])
                .build();
            let fill = gtk::Box::builder()
                .css_classes(["poster-progress-fill"])
                .hexpand(false)
                .build();
            fill.set_size_request((130.0 * fraction) as i32, -1);
            track.append(&fill);
            card.append(&track);
        }
    }

    let title = gtk::Label::new(Some(&item.display_title()));
    title.set_ellipsize(pango::EllipsizeMode::End);
    title.set_max_width_chars(15);
    title.add_css_class("poster-title");
    title.set_halign(gtk::Align::Start);
    card.append(&title);

    let sub = match item.item_type.as_deref() {
        Some("episode") => {
            let show = item.grandparent_title.as_deref().unwrap_or("");
            let s = item.parent_index.unwrap_or(0);
            let e = item.index.unwrap_or(0);
            format!("{} \u{00b7} S{:02}E{:02}", show, s, e)
        }
        _ => item.year.map(|y| y.to_string()).unwrap_or_default(),
    };
    if !sub.is_empty() {
        let sub_label = gtk::Label::new(Some(&sub));
        sub_label.set_ellipsize(pango::EllipsizeMode::End);
        sub_label.set_max_width_chars(15);
        sub_label.add_css_class("poster-subtitle");
        sub_label.set_halign(gtk::Align::Start);
        card.append(&sub_label);
    }

    card
}

pub fn handle_item_click(
    item: &MediaItem,
    nav_view: &adw::NavigationView,
    state: &AppState,
    toast_overlay: &adw::ToastOverlay,
    window: &adw::ApplicationWindow,
) {
    match item.item_type.as_deref() {
        Some("movie") | Some("episode") => {
            let detail_page = detail::build_detail_page(item, state, toast_overlay);
            nav_view.push(&detail_page);
        }
        Some("show") => {
            let Some(rk) = &item.rating_key else { return };
            let rk = rk.clone();
            let title = item.display_title();

            let plex = {
                let c = state.client.borrow();
                match c.as_ref() {
                    Some(p) => p.clone(),
                    None => return,
                }
            };

            let nav_view = nav_view.clone();
            let state = state.clone();
            let toast_overlay = toast_overlay.clone();
            let window = window.clone();

            util::spawn_async(&state, async move {
                plex.get_children(&rk).await.unwrap_or_default()
            }, move |seasons, state| {
                let page = super::seasons::build_seasons_page(
                    &title, &seasons, &nav_view, &state, &toast_overlay, &window,
                );
                nav_view.push(&page);
            });
        }
        Some("season") => {
            let Some(rk) = &item.rating_key else { return };
            let rk = rk.clone();
            let title = item.display_title();

            let plex = {
                let c = state.client.borrow();
                match c.as_ref() {
                    Some(p) => p.clone(),
                    None => return,
                }
            };

            let nav_view = nav_view.clone();
            let state = state.clone();
            let toast_overlay = toast_overlay.clone();

            util::spawn_async(&state, async move {
                plex.get_children(&rk).await.unwrap_or_default()
            }, move |episodes, state| {
                let page = super::episodes::build_episodes_page(
                    &title, &episodes, &state, &toast_overlay,
                );
                nav_view.push(&page);
            });
        }
        _ => {}
    }
}
