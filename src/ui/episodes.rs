use gtk4 as gtk;
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

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    vbox.set_margin_start(24);
    vbox.set_margin_end(24);
    vbox.set_margin_top(16);

    let listbox = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    for ep in episodes {
        let ep_num = ep.index.unwrap_or(0);
        let title_str = format!("{}. {}", ep_num, ep.display_title());

        let row = adw::ActionRow::builder()
            .title(&title_str)
            .activatable(true)
            .build();

        if let Some(d) = ep.duration {
            row.set_subtitle(&util::format_duration(d));
        }

        if let Some(thumb) = &ep.thumb {
            let pic = gtk::Picture::builder()
                .width_request(120)
                .height_request(68)
                .css_classes(["poster-image"])
                .build();
            let c = state.client.borrow();
            if let Some(ref plex) = *c {
                let url = plex.poster_url(thumb);
                util::load_image_async(&pic, &url, state.image_cache.clone(), plex.http.clone());
            }
            row.add_prefix(&pic);
        }

        let play_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
        row.add_suffix(&play_icon);

        let state = state.clone();
        let toast_overlay = toast_overlay.clone();
        let part_key = ep.stream_part_key().map(|s| s.to_string());
        let ep_title = ep.display_title();
        let rating_key = ep.rating_key.clone();
        let duration = ep.duration;

        row.connect_activated(move |_| {
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

        listbox.append(&row);
    }

    vbox.append(&listbox);
    scroll.set_child(Some(&vbox));
    toolbar.set_content(Some(&scroll));

    adw::NavigationPage::builder()
        .title(season_title)
        .child(&toolbar)
        .build()
}
