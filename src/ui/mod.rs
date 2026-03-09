mod detail;
mod episodes;
mod grid;
mod login;
mod playback;
pub mod player_widget;
mod seasons;
mod settings;
mod sidebar;
mod state;
mod style;
mod util;

use gtk4 as gtk;
use gtk4::glib;
use libadwaita as adw;

use adw::prelude::*;

use crate::config::Config;
use crate::plex::PlexClient;

use state::AppState;

pub fn build_ui(app: &adw::Application, rt: tokio::runtime::Handle) {
    style::load_css();

    let config = Config::load();

    let main_stack = gtk::Stack::new();
    main_stack.set_transition_type(gtk::StackTransitionType::None);

    let hwdec_value = config.hwdec.as_mpv_value().to_string();
    let player_widget = player_widget::PlayerWidget::new(&hwdec_value, config.seek_seconds);
    main_stack.add_named(&player_widget.widget, Some("player"));

    player_widget.set_on_stop({
        let main_stack = main_stack.clone();
        move || {
            main_stack.set_visible_child_name("main");
        }
    });

    let (sidebar_box, sidebar_list, settings_btn) = sidebar::build_sidebar();
    let state = AppState::new(rt, player_widget, main_stack.clone(), config.clone(), sidebar_list.clone());

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Plex")
        .default_width(1280)
        .default_height(800)
        .build();

    let toast_overlay = adw::ToastOverlay::new();

    // Login page
    let login_page = login::build_login_page(&state, &main_stack, &toast_overlay);
    main_stack.add_named(&login_page, Some("login"));

    // Main view (sidebar + content)
    let main_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    main_box.append(&sidebar_box);
    main_box.append(&gtk::Separator::new(gtk::Orientation::Vertical));

    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content_box.set_hexpand(true);
    content_box.set_vexpand(true);

    let nav_view = adw::NavigationView::new();
    content_box.append(&nav_view);

    let content_toolbar = adw::ToolbarView::new();
    let content_header = adw::HeaderBar::new();

    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Search library...")
        .build();
    search_entry.set_size_request(250, -1);
    content_header.pack_end(&search_entry);

    content_toolbar.add_top_bar(&content_header);

    let content_stack = gtk::Stack::new();
    content_stack.set_transition_type(gtk::StackTransitionType::None);

    // Loading spinner
    let loading_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    loading_box.set_valign(gtk::Align::Center);
    loading_box.set_halign(gtk::Align::Center);
    let spinner = gtk::Spinner::new();
    spinner.set_size_request(48, 48);
    spinner.start();
    loading_box.append(&spinner);
    loading_box.append(&gtk::Label::new(Some("Loading...")));
    content_stack.add_named(&loading_box, Some("loading"));

    // Poster grid
    let grid_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    let flow_box = gtk::FlowBox::new();
    flow_box.set_homogeneous(true);
    flow_box.set_max_children_per_line(8);
    flow_box.set_min_children_per_line(2);
    flow_box.set_selection_mode(gtk::SelectionMode::None);
    flow_box.set_column_spacing(4);
    flow_box.set_row_spacing(4);
    flow_box.set_margin_start(16);
    flow_box.set_margin_end(16);
    flow_box.set_margin_top(8);
    flow_box.set_margin_bottom(16);
    flow_box.set_valign(gtk::Align::Start);
    grid_scroll.set_child(Some(&flow_box));
    content_stack.add_named(&grid_scroll, Some("grid"));

    // Empty state
    let empty_page = adw::StatusPage::builder()
        .title("No Items")
        .description("Select a library from the sidebar")
        .icon_name("folder-videos-symbolic")
        .build();
    content_stack.add_named(&empty_page, Some("empty"));

    content_toolbar.set_content(Some(&content_stack));

    let root_page = adw::NavigationPage::builder()
        .title("Home")
        .child(&content_toolbar)
        .build();
    nav_view.push(&root_page);

    // --- Settings button ---
    {
        let nav_view = nav_view.clone();
        let state = state.clone();
        settings_btn.connect_clicked(move |_| {
            let page = settings::build_settings_page(&state);
            nav_view.push(&page);
        });
    }

    main_box.append(&content_box);
    main_stack.add_named(&main_box, Some("main"));

    // --- Sidebar selection ---
    {
        let state = state.clone();
        let flow_box = flow_box.clone();
        let content_stack = content_stack.clone();
        let nav_view = nav_view.clone();
        let root_page = root_page.clone();
        let toast_overlay = toast_overlay.clone();
        let window = window.clone();

        sidebar_list.connect_row_selected(move |_, row: Option<&gtk::ListBoxRow>| {
            let Some(row) = row else { return };
            let idx = row.index();
            nav_view.pop_to_page(&root_page);

            let plex = {
                let c = state.client.borrow();
                match c.as_ref() {
                    Some(p) => p.clone(),
                    None => return,
                }
            };

            content_stack.set_visible_child_name("loading");

            if idx == 0 {
                root_page.set_title("Home");
                let flow_box = flow_box.clone();
                let content_stack = content_stack.clone();
                let nav_view = nav_view.clone();
                let toast_overlay = toast_overlay.clone();
                let window = window.clone();
                let state = state.clone();

                util::spawn_async(&state, async move {
                    let mut items = Vec::new();
                    if let Ok(deck) = plex.get_on_deck().await {
                        items.extend(deck);
                    }
                    if let Ok(recent) = plex.get_recently_added().await {
                        items.extend(recent);
                    }
                    items
                }, move |items, state| {
                    grid::populate_grid(
                        &flow_box, &items, &state,
                        &nav_view, &toast_overlay, &window,
                    );
                    if items.is_empty() {
                        content_stack.set_visible_child_name("empty");
                    } else {
                        content_stack.set_visible_child_name("grid");
                    }
                });
            } else {
                let lib_title = row
                    .child()
                    .and_then(|c: gtk::Widget| c.downcast::<gtk::Label>().ok())
                    .map(|l: gtk::Label| l.text().to_string())
                    .unwrap_or_default();
                root_page.set_title(&lib_title);
                let lib_key = row.widget_name().to_string();

                let flow_box = flow_box.clone();
                let content_stack = content_stack.clone();
                let nav_view = nav_view.clone();
                let toast_overlay = toast_overlay.clone();
                let window = window.clone();
                let state = state.clone();

                util::spawn_async(&state, async move {
                    plex.get_library_items(&lib_key).await
                }, move |result, state| {
                    match result {
                        Ok(items) => {
                            grid::populate_grid(
                                &flow_box, &items, &state,
                                &nav_view, &toast_overlay, &window,
                            );
                            if items.is_empty() {
                                content_stack.set_visible_child_name("empty");
                            } else {
                                content_stack.set_visible_child_name("grid");
                            }
                        }
                        Err(e) => {
                            content_stack.set_visible_child_name("empty");
                            toast_overlay.add_toast(adw::Toast::new(&format!("Error: {}", e)));
                        }
                    }
                });
            }
        });
    }

    // --- Search ---
    {
        let state = state.clone();
        let flow_box = flow_box.clone();
        let content_stack = content_stack.clone();
        let nav_view = nav_view.clone();
        let root_page = root_page.clone();
        let toast_overlay = toast_overlay.clone();
        let window = window.clone();

        search_entry.connect_activate(move |entry| {
            let query = entry.text().to_string();
            if query.trim().is_empty() {
                return;
            }

            nav_view.pop_to_page(&root_page);
            root_page.set_title(&format!("Search: {}", query));
            content_stack.set_visible_child_name("loading");

            let plex = {
                let c = state.client.borrow();
                match c.as_ref() {
                    Some(p) => p.clone(),
                    None => return,
                }
            };

            let flow_box = flow_box.clone();
            let content_stack = content_stack.clone();
            let nav_view = nav_view.clone();
            let toast_overlay = toast_overlay.clone();
            let window = window.clone();
            let state = state.clone();

            util::spawn_async(&state, async move {
                plex.search(&query).await
            }, move |result, state| {
                match result {
                    Ok(items) => {
                        grid::populate_grid(
                            &flow_box, &items, &state,
                            &nav_view, &toast_overlay, &window,
                        );
                        if items.is_empty() {
                            content_stack.set_visible_child_name("empty");
                        } else {
                            content_stack.set_visible_child_name("grid");
                        }
                    }
                    Err(e) => {
                        content_stack.set_visible_child_name("empty");
                        toast_overlay.add_toast(adw::Toast::new(&format!("Search error: {}", e)));
                    }
                }
            });
        });
    }

    // --- Loading page shown during auto-reconnect ---
    let loading_page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    loading_page.set_vexpand(true);
    loading_page.set_hexpand(true);
    loading_page.set_valign(gtk::Align::Center);
    loading_page.set_halign(gtk::Align::Center);
    let reconnect_spinner = gtk::Spinner::new();
    reconnect_spinner.set_size_request(48, 48);
    reconnect_spinner.start();
    loading_page.append(&reconnect_spinner);
    loading_page.append(&gtk::Label::new(Some("Connecting to Plex...")));
    main_stack.add_named(&loading_page, Some("connecting"));

    // Set initial view and present window IMMEDIATELY so user sees the UI
    if config.is_configured() {
        main_stack.set_visible_child_name("connecting");
    } else {
        main_stack.set_visible_child_name("login");
    }

    toast_overlay.set_child(Some(&main_stack));
    window.set_content(Some(&toast_overlay));
    window.present();

    // --- Auto-connect AFTER window is on screen ---
    if config.is_configured() {
        let url = config.server_url.clone().unwrap();
        let token = config.token.clone().unwrap();
        let client_id = config.client_id.clone();

        let sidebar_list = sidebar_list.clone();
        let main_stack_c = main_stack.clone();
        let toast_overlay_c = toast_overlay.clone();

        glib::idle_add_local_once({
            let state = state.clone();
            move || {
                util::spawn_async(&state, async move {
                    let c = PlexClient::connect(&url, &token, &client_id).await?;
                    let libs = c.get_libraries().await.unwrap_or_default();
                    Ok::<_, anyhow::Error>((c, libs))
                }, move |result, state| {
                    match result {
                        Ok((plex, libs)) => {
                            *state.client.borrow_mut() = Some(plex);
                            sidebar::populate_sidebar(&sidebar_list, &libs);
                            main_stack_c.set_visible_child_name("main");
                        }
                        Err(e) => {
                            main_stack_c.set_visible_child_name("login");
                            toast_overlay_c.add_toast(adw::Toast::new(&format!(
                                "Reconnect failed: {}. Please sign in again.", e
                            )));
                        }
                    }
                });
            }
        });
    }
}
