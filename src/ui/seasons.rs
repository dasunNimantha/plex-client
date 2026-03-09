use gtk4 as gtk;
use libadwaita as adw;

use adw::prelude::*;

use crate::plex::MediaItem;

use super::state::AppState;
use super::util;

pub fn build_seasons_page(
    show_title: &str,
    seasons: &[MediaItem],
    nav_view: &adw::NavigationView,
    state: &AppState,
    toast_overlay: &adw::ToastOverlay,
    _window: &adw::ApplicationWindow,
) -> adw::NavigationPage {
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 4);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(24);

    for season in seasons {
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        row_box.set_margin_start(16);
        row_box.set_margin_end(16);
        row_box.set_margin_top(10);
        row_box.set_margin_bottom(10);
        row_box.set_valign(gtk::Align::Center);
        row_box.add_css_class("ep-row");

        if let Some(thumb) = &season.thumb {
            let pic = gtk::Picture::builder()
                .width_request(80)
                .height_request(120)
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
            .label(&season.display_title())
            .halign(gtk::Align::Start)
            .css_classes(["ep-title"])
            .build();
        text_col.append(&title_label);

        if let Some(lc) = season.leaf_count {
            let count_label = gtk::Label::builder()
                .label(&format!("{} episodes", lc))
                .halign(gtk::Align::Start)
                .css_classes(["ep-duration"])
                .build();
            text_col.append(&count_label);
        }

        row_box.append(&text_col);

        let arrow = gtk::Image::from_icon_name("go-next-symbolic");
        arrow.set_pixel_size(16);
        arrow.set_valign(gtk::Align::Center);
        arrow.add_css_class("ep-play-icon");
        row_box.append(&arrow);

        let event_row = gtk::Button::builder()
            .child(&row_box)
            .css_classes(["flat"])
            .build();

        let rk = season.rating_key.clone();
        let title = season.display_title();
        let nav_view = nav_view.clone();
        let state = state.clone();
        let toast_overlay = toast_overlay.clone();

        event_row.connect_clicked(move |_| {
            let Some(ref rk) = rk else { return };
            let rk = rk.clone();

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
            let title = title.clone();

            util::spawn_async(&state, async move {
                plex.get_children(&rk).await.unwrap_or_default()
            }, move |episodes, state| {
                let page = super::episodes::build_episodes_page(
                    &title, &episodes, &state, &toast_overlay,
                );
                nav_view.push(&page);
            });
        });

        vbox.append(&event_row);
    }

    scroll.set_child(Some(&vbox));
    toolbar.set_content(Some(&scroll));

    adw::NavigationPage::builder()
        .title(show_title)
        .child(&toolbar)
        .build()
}
