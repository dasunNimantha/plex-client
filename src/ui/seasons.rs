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

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    vbox.set_margin_start(24);
    vbox.set_margin_end(24);
    vbox.set_margin_top(16);

    let listbox = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    for season in seasons {
        let row = adw::ActionRow::builder()
            .title(&season.display_title())
            .activatable(true)
            .build();

        if let Some(lc) = season.leaf_count {
            row.set_subtitle(&format!("{} episodes", lc));
        }

        row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));

        let rk = season.rating_key.clone();
        let title = season.display_title();
        let nav_view = nav_view.clone();
        let state = state.clone();
        let toast_overlay = toast_overlay.clone();

        row.connect_activated(move |_| {
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

        listbox.append(&row);
    }

    vbox.append(&listbox);
    scroll.set_child(Some(&vbox));
    toolbar.set_content(Some(&scroll));

    adw::NavigationPage::builder()
        .title(show_title)
        .child(&toolbar)
        .build()
}
