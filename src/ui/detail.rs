use gtk4 as gtk;
use libadwaita as adw;

use adw::prelude::*;

use crate::plex::MediaItem;

use super::playback;
use super::state::AppState;
use super::util;

pub fn build_detail_page(
    item: &MediaItem,
    state: &AppState,
    toast_overlay: &adw::ToastOverlay,
) -> adw::NavigationPage {
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 24);
    content.set_margin_start(32);
    content.set_margin_end(32);
    content.set_margin_top(24);
    content.set_margin_bottom(24);

    let poster = gtk::Picture::builder()
        .width_request(250)
        .height_request(375)
        .css_classes(["poster-image"])
        .valign(gtk::Align::Start)
        .build();

    if let Some(thumb) = &item.thumb {
        let c = state.client.borrow();
        if let Some(ref plex) = *c {
            let url = plex.poster_url_full(thumb);
            util::load_image_async(&poster, &url, state.image_cache.clone(), plex.http.clone());
        }
    }
    content.append(&poster);

    let info = gtk::Box::new(gtk::Orientation::Vertical, 8);
    info.set_hexpand(true);
    info.set_valign(gtk::Align::Start);

    let title_label = gtk::Label::builder()
        .label(&item.display_title())
        .halign(gtk::Align::Start)
        .wrap(true)
        .css_classes(["detail-title"])
        .build();
    info.append(&title_label);

    let mut meta_parts = Vec::new();
    if let Some(y) = item.year {
        meta_parts.push(y.to_string());
    }
    if let Some(cr) = &item.content_rating {
        meta_parts.push(cr.clone());
    }
    if let Some(d) = item.duration {
        meta_parts.push(util::format_duration(d));
    }
    if let Some(r) = item.audience_rating.or(item.rating) {
        meta_parts.push(format!("\u{2605} {:.1}", r));
    }
    if !meta_parts.is_empty() {
        let meta_label = gtk::Label::builder()
            .label(&meta_parts.join("  \u{00b7}  "))
            .halign(gtk::Align::Start)
            .css_classes(["detail-meta"])
            .build();
        info.append(&meta_label);
    }

    if item.item_type.as_deref() == Some("episode") {
        let mut ep_info = String::new();
        if let Some(show) = &item.grandparent_title {
            ep_info.push_str(show);
        }
        if let Some(s) = item.parent_index {
            if let Some(e) = item.index {
                ep_info.push_str(&format!(" \u{00b7} S{:02}E{:02}", s, e));
            }
        }
        if !ep_info.is_empty() {
            let ep_label = gtk::Label::builder()
                .label(&ep_info)
                .halign(gtk::Align::Start)
                .css_classes(["detail-meta"])
                .build();
            info.append(&ep_label);
        }
    }

    let play_btn = gtk::Button::builder()
        .label("\u{25b6}  Play")
        .css_classes(["suggested-action", "pill"])
        .halign(gtk::Align::Start)
        .margin_top(16)
        .margin_bottom(16)
        .build();

    {
        let state = state.clone();
        let toast_overlay = toast_overlay.clone();
        let part_key = item.stream_part_key().map(|s| s.to_string());
        let title = item.display_title();
        let rating_key = item.rating_key.clone();
        let duration = item.duration;

        play_btn.connect_clicked(move |_| {
            let Some(ref pk) = part_key else {
                toast_overlay.add_toast(adw::Toast::new("No playable media found"));
                return;
            };
            let c = state.client.borrow();
            let Some(ref plex) = *c else { return };
            let url = plex.stream_url(pk);
            drop(c);

            state.main_stack.set_visible_child_name("player");
            state.player_widget.play(&url, &title);
            playback::start_progress_tracking(&state, rating_key.clone(), duration);
        });
    }
    info.append(&play_btn);

    if let Some(summary) = &item.summary {
        if !summary.is_empty() {
            let summary_label = gtk::Label::builder()
                .label(summary)
                .halign(gtk::Align::Start)
                .wrap(true)
                .css_classes(["detail-summary"])
                .build();
            info.append(&summary_label);
        }
    }

    let media_info = item.media_info_string();
    if !media_info.is_empty() {
        let mi_label = gtk::Label::builder()
            .label(&media_info)
            .halign(gtk::Align::Start)
            .margin_top(16)
            .css_classes(["detail-media-info"])
            .build();
        info.append(&mi_label);
    }

    content.append(&info);
    scroll.set_child(Some(&content));
    toolbar.set_content(Some(&scroll));

    adw::NavigationPage::builder()
        .title(&item.display_title())
        .child(&toolbar)
        .build()
}
