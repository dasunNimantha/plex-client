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
    let header = adw::HeaderBar::new();
    header.add_css_class("flat");
    toolbar.add_top_bar(&header);

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.add_css_class("detail-page-bg");

    let hero = gtk::Overlay::new();
    hero.set_size_request(-1, 380);
    hero.set_overflow(gtk::Overflow::Hidden);

    if let Some(art) = &item.art {
        let c = state.client.borrow();
        if let Some(ref plex) = *c {
            let art_url = plex.art_url(art, 1920, 1080);
            let backdrop = gtk::Picture::builder()
                .content_fit(gtk::ContentFit::Cover)
                .css_classes(["detail-backdrop"])
                .hexpand(true)
                .vexpand(true)
                .build();
            util::load_image_async(&backdrop, &art_url, state.image_cache.clone(), plex.http.clone());
            hero.set_child(Some(&backdrop));
        }
    }

    let gradient = gtk::Box::builder()
        .css_classes(["detail-gradient"])
        .hexpand(true)
        .vexpand(true)
        .build();
    hero.add_overlay(&gradient);

    let overlay_box = gtk::Box::new(gtk::Orientation::Horizontal, 24);
    overlay_box.set_margin_start(40);
    overlay_box.set_margin_end(40);
    overlay_box.set_margin_bottom(32);
    overlay_box.set_valign(gtk::Align::End);
    overlay_box.set_halign(gtk::Align::Fill);

    let poster = gtk::Picture::builder()
        .width_request(150)
        .height_request(225)
        .css_classes(["detail-poster"])
        .valign(gtk::Align::End)
        .build();

    if let Some(thumb) = &item.thumb {
        let c = state.client.borrow();
        if let Some(ref plex) = *c {
            let url = plex.poster_url_full(thumb);
            util::load_image_async(&poster, &url, state.image_cache.clone(), plex.http.clone());
        }
    }
    overlay_box.append(&poster);

    let info = gtk::Box::new(gtk::Orientation::Vertical, 4);
    info.set_hexpand(true);
    info.set_valign(gtk::Align::End);

    let title_label = gtk::Label::builder()
        .label(&item.display_title())
        .halign(gtk::Align::Start)
        .wrap(true)
        .css_classes(["detail-title"])
        .build();
    info.append(&title_label);

    let meta_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    meta_box.set_margin_top(6);

    if let Some(y) = item.year {
        let yl = gtk::Label::builder()
            .label(&y.to_string())
            .css_classes(["detail-meta"])
            .build();
        meta_box.append(&yl);
    }
    if let Some(cr) = &item.content_rating {
        let tag = gtk::Label::builder()
            .label(cr)
            .css_classes(["detail-meta-tag"])
            .build();
        meta_box.append(&tag);
    }
    if let Some(d) = item.duration {
        let dl = gtk::Label::builder()
            .label(&util::format_duration(d))
            .css_classes(["detail-meta"])
            .build();
        meta_box.append(&dl);
    }
    if let Some(r) = item.audience_rating.or(item.rating) {
        let rl = gtk::Label::builder()
            .label(&format!("\u{2605} {:.1}", r))
            .css_classes(["detail-meta"])
            .build();
        meta_box.append(&rl);
    }

    info.append(&meta_box);

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
        .halign(gtk::Align::Start)
        .margin_top(14)
        .css_classes(["plex-play-btn"])
        .build();
    let play_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let play_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
    play_icon.set_pixel_size(16);
    play_content.append(&play_icon);
    play_content.append(&gtk::Label::new(Some("Play")));
    play_btn.set_child(Some(&play_content));

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

    overlay_box.append(&info);
    hero.add_overlay(&overlay_box);
    outer.append(&hero);

    let details_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    details_box.set_margin_start(40);
    details_box.set_margin_end(40);
    details_box.set_margin_top(24);
    details_box.set_margin_bottom(40);

    if let Some(summary) = &item.summary {
        if !summary.is_empty() {
            let summary_label = gtk::Label::builder()
                .label(summary)
                .halign(gtk::Align::Start)
                .wrap(true)
                .css_classes(["detail-summary"])
                .build();
            details_box.append(&summary_label);
        }
    }

    let media_info = item.media_info_string();
    if !media_info.is_empty() {
        let mi_label = gtk::Label::builder()
            .label(&media_info)
            .halign(gtk::Align::Start)
            .margin_top(6)
            .css_classes(["detail-media-info"])
            .build();
        details_box.append(&mi_label);
    }

    outer.append(&details_box);
    scroll.set_child(Some(&outer));
    toolbar.set_content(Some(&scroll));

    adw::NavigationPage::builder()
        .title(&item.display_title())
        .child(&toolbar)
        .build()
}
