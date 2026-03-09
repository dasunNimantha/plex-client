use gtk4 as gtk;
use gtk4::pango;
use libadwaita as adw;

use gtk::prelude::*;

use crate::plex::MediaItem;

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

    // Build all cards then add in one pass to reduce layout recalcs
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

fn create_poster_card(
    item: &MediaItem,
    poster_url: Option<String>,
    image_cache: &super::state::ImageCache,
    http: &reqwest::Client,
) -> gtk::Box {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .css_classes(["poster-card"])
        .halign(gtk::Align::Center)
        .build();

    let picture = gtk::Picture::builder()
        .width_request(150)
        .height_request(225)
        .content_fit(gtk::ContentFit::Cover)
        .css_classes(["poster-image"])
        .build();

    if let Some(url) = poster_url {
        util::load_image_async(&picture, &url, image_cache.clone(), http.clone());
    }

    card.append(&picture);

    let title = gtk::Label::new(Some(&item.display_title()));
    title.set_ellipsize(pango::EllipsizeMode::End);
    title.set_max_width_chars(20);
    title.add_css_class("poster-title");
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
        sub_label.set_max_width_chars(20);
        sub_label.add_css_class("poster-subtitle");
        card.append(&sub_label);
    }

    card
}

fn handle_item_click(
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
