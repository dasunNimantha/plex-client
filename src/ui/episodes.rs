use gtk4 as gtk;
use gtk4::pango;
use libadwaita as adw;

use adw::prelude::*;

use crate::plex::MediaItem;

use super::playback;
use super::state::AppState;
use super::util;

pub fn build_episodes_page(
    season_title: &str,
    episodes: &[MediaItem],
    state: &AppState,
    toast_overlay: &adw::ToastOverlay,
) -> adw::NavigationPage {
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 6);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(24);

    for ep in episodes {
        let ep_num = ep.index.unwrap_or(0);

        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row_box.set_margin_start(12);
        row_box.set_margin_end(12);
        row_box.set_margin_top(10);
        row_box.set_margin_bottom(10);
        row_box.set_valign(gtk::Align::Center);
        row_box.add_css_class("ep-row");

        let num_label = gtk::Label::builder()
            .label(&ep_num.to_string())
            .halign(gtk::Align::Center)
            .css_classes(["ep-num"])
            .build();
        row_box.append(&num_label);

        if let Some(thumb) = &ep.thumb {
            let pic = gtk::Picture::builder()
                .width_request(160)
                .height_request(90)
                .content_fit(gtk::ContentFit::Cover)
                .css_classes(["poster-image"])
                .build();
            let c = state.client.borrow();
            if let Some(ref plex) = *c {
                let url = plex.poster_url(thumb);
                util::load_image_async(&pic, &url, state.image_cache.clone(), plex.http.clone());
            }
            row_box.append(&pic);
        }

        let text_col = gtk::Box::new(gtk::Orientation::Vertical, 2);
        text_col.set_hexpand(true);
        text_col.set_valign(gtk::Align::Center);

        let title_label = gtk::Label::builder()
            .label(&ep.display_title())
            .halign(gtk::Align::Start)
            .ellipsize(pango::EllipsizeMode::End)
            .css_classes(["ep-title"])
            .build();
        text_col.append(&title_label);

        let mut meta_parts = Vec::new();
        if let Some(d) = ep.duration {
            meta_parts.push(util::format_duration(d));
        }
        if let Some(y) = ep.year {
            meta_parts.push(y.to_string());
        }
        if !meta_parts.is_empty() {
            let dur_label = gtk::Label::builder()
                .label(&meta_parts.join(" \u{00b7} "))
                .halign(gtk::Align::Start)
                .css_classes(["ep-duration"])
                .build();
            text_col.append(&dur_label);
        }

        if let Some(summary) = &ep.summary {
            if !summary.is_empty() {
                let desc = gtk::Label::builder()
                    .label(summary)
                    .halign(gtk::Align::Start)
                    .ellipsize(pango::EllipsizeMode::End)
                    .max_width_chars(60)
                    .lines(2)
                    .wrap(true)
                    .wrap_mode(pango::WrapMode::WordChar)
                    .css_classes(["ep-desc"])
                    .build();
                text_col.append(&desc);
            }
        }

        row_box.append(&text_col);

        let play_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
        play_icon.set_pixel_size(20);
        play_icon.set_valign(gtk::Align::Center);
        play_icon.add_css_class("ep-play-icon");
        row_box.append(&play_icon);

        let event_row = gtk::Button::builder()
            .child(&row_box)
            .css_classes(["flat"])
            .build();

        let state = state.clone();
        let toast_overlay = toast_overlay.clone();
        let part_key = ep.stream_part_key().map(|s| s.to_string());
        let ep_title = ep.display_title();
        let rating_key = ep.rating_key.clone();
        let duration = ep.duration;

        event_row.connect_clicked(move |_| {
            let Some(ref pk) = part_key else {
                toast_overlay.add_toast(adw::Toast::new("No playable media"));
                return;
            };
            let c = state.client.borrow();
            let Some(ref plex) = *c else { return };
            let url = plex.stream_url(pk);
            drop(c);

            state.main_stack.set_visible_child_name("player");
            state.player_widget.play(&url, &ep_title);
            playback::start_progress_tracking(&state, rating_key.clone(), duration);
        });

        vbox.append(&event_row);
    }

    scroll.set_child(Some(&vbox));
    toolbar.set_content(Some(&scroll));

    adw::NavigationPage::builder()
        .title(season_title)
        .child(&toolbar)
        .build()
}
