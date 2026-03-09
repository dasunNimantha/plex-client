use gtk4 as gtk;
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::pango;
use libadwaita as adw;

use adw::prelude::*;
use gtk::prelude::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use crate::config::Config;
use crate::player::MpvPlayer;
use crate::plex::{Library, MediaItem, PlexClient};

type ImageCache = Rc<RefCell<HashMap<String, gdk::Texture>>>;
type Client = Rc<RefCell<Option<PlexClient>>>;
type Player = Rc<RefCell<MpvPlayer>>;
type Items = Rc<RefCell<Vec<MediaItem>>>;

const CSS: &str = r#"
.sidebar {
    background-color: alpha(@window_bg_color, 0.97);
}
.sidebar-row {
    padding: 12px 16px;
    font-size: 14px;
}
.sidebar-row-label {
    font-weight: 500;
}
.poster-card {
    padding: 6px;
    border-radius: 12px;
    transition: background-color 200ms ease;
}
.poster-card:hover {
    background-color: alpha(currentColor, 0.07);
}
.poster-image {
    border-radius: 8px;
    background-color: alpha(currentColor, 0.04);
}
.poster-title {
    font-size: 13px;
    font-weight: 500;
    margin-top: 4px;
}
.poster-subtitle {
    font-size: 11px;
    opacity: 0.6;
}
.detail-title {
    font-size: 28px;
    font-weight: bold;
}
.detail-meta {
    font-size: 14px;
    opacity: 0.7;
}
.detail-summary {
    font-size: 14px;
}
.detail-media-info {
    font-size: 12px;
    opacity: 0.5;
    font-family: monospace;
}
.section-title {
    font-size: 20px;
    font-weight: bold;
    margin-top: 16px;
    margin-bottom: 8px;
}
.login-title {
    font-size: 32px;
    font-weight: bold;
}
.login-subtitle {
    font-size: 14px;
    opacity: 0.6;
}
.play-button {
    padding: 12px 32px;
    font-size: 16px;
    font-weight: bold;
}
"#;

/// Runs a closure on a background thread and delivers the result to a callback on the GTK main thread.
fn spawn_task<T: Send + 'static>(
    task: impl FnOnce() -> T + Send + 'static,
    callback: impl FnOnce(T) + 'static,
) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = task();
        let _ = tx.send(result);
    });
    let callback = RefCell::new(Some(callback));
    glib::timeout_add_local(Duration::from_millis(30), move || match rx.try_recv() {
        Ok(result) => {
            if let Some(cb) = callback.borrow_mut().take() {
                cb(result);
            }
            glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
    });
}

/// Load image bytes into a gdk::Texture via PixbufLoader.
fn texture_from_bytes(bytes: &[u8]) -> Option<gdk::Texture> {
    let loader = gdk_pixbuf::PixbufLoader::new();
    loader.write(bytes).ok()?;
    loader.close().ok()?;
    let pixbuf = loader.pixbuf()?;
    Some(gdk::Texture::for_pixbuf(&pixbuf))
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn build_ui(app: &adw::Application) {
    load_css();

    let config = Config::load();
    let image_cache: ImageCache = Rc::new(RefCell::new(HashMap::new()));
    let player: Player = Rc::new(RefCell::new(MpvPlayer::new()));
    let client: Client = Rc::new(RefCell::new(None));
    let current_items: Items = Rc::new(RefCell::new(Vec::new()));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Plex")
        .default_width(1280)
        .default_height(800)
        .build();

    let toast_overlay = adw::ToastOverlay::new();

    let main_stack = gtk::Stack::new();
    main_stack.set_transition_type(gtk::StackTransitionType::Crossfade);

    // ============================
    // Login Page
    // ============================

    let login_page = build_login_page(
        client.clone(),
        main_stack.clone(),
        toast_overlay.clone(),
    );
    main_stack.add_named(&login_page, Some("login"));

    // ============================
    // Main View
    // ============================

    let main_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    // --- Sidebar ---
    let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar_box.set_size_request(240, -1);
    sidebar_box.add_css_class("sidebar");

    let sidebar_header = adw::HeaderBar::builder()
        .title_widget(&gtk::Label::new(Some("Libraries")))
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();
    sidebar_box.append(&sidebar_header);

    let sidebar_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    let sidebar_list = gtk::ListBox::new();
    sidebar_list.set_selection_mode(gtk::SelectionMode::Single);
    sidebar_list.add_css_class("navigation-sidebar");
    sidebar_scroll.set_child(Some(&sidebar_list));
    sidebar_box.append(&sidebar_scroll);

    main_box.append(&sidebar_box);
    main_box.append(&gtk::Separator::new(gtk::Orientation::Vertical));

    // --- Content Area ---
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
    content_stack.set_transition_type(gtk::StackTransitionType::Crossfade);

    // Loading state
    let loading_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    loading_box.set_valign(gtk::Align::Center);
    loading_box.set_halign(gtk::Align::Center);
    let spinner = gtk::Spinner::new();
    spinner.set_size_request(48, 48);
    spinner.start();
    loading_box.append(&spinner);
    loading_box.append(&gtk::Label::new(Some("Loading...")));
    content_stack.add_named(&loading_box, Some("loading"));

    // Grid state
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

    main_box.append(&content_box);
    main_stack.add_named(&main_box, Some("main"));

    // ============================
    // Sidebar selection handler
    // ============================

    {
        let client = client.clone();
        let flow_box = flow_box.clone();
        let content_stack = content_stack.clone();
        let current_items = current_items.clone();
        let image_cache = image_cache.clone();
        let nav_view = nav_view.clone();
        let root_page = root_page.clone();
        let player = player.clone();
        let toast_overlay = toast_overlay.clone();
        let window = window.clone();

        sidebar_list.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let idx = row.index();

            nav_view.pop_to_page(&root_page);

            let c = client.borrow();
            let Some(ref plex) = *c else { return };
            let plex = plex.clone();
            drop(c);

            content_stack.set_visible_child_name("loading");

            if idx == 0 {
                // Home
                root_page.set_title("Home");

                let flow_box = flow_box.clone();
                let content_stack = content_stack.clone();
                let current_items = current_items.clone();
                let image_cache = image_cache.clone();
                let nav_view = nav_view.clone();
                let player = player.clone();
                let toast_overlay = toast_overlay.clone();
                let window = window.clone();
                let client = client.clone();

                spawn_task(
                    move || {
                        let mut items = Vec::new();
                        if let Ok(deck) = plex.get_on_deck() {
                            items.extend(deck);
                        }
                        if let Ok(recent) = plex.get_recently_added() {
                            items.extend(recent);
                        }
                        items
                    },
                    move |items| {
                        populate_grid(
                            &flow_box,
                            &items,
                            &current_items,
                            &image_cache,
                            &client,
                            &nav_view,
                            &player,
                            &toast_overlay,
                            &window,
                        );
                        if items.is_empty() {
                            content_stack.set_visible_child_name("empty");
                        } else {
                            content_stack.set_visible_child_name("grid");
                        }
                    },
                );
            } else {
                // Library selected
                let lib_title = row
                    .child()
                    .and_then(|c| c.downcast::<gtk::Label>().ok())
                    .map(|l| l.text().to_string())
                    .unwrap_or_default();
                root_page.set_title(&lib_title);
                let lib_key = row.widget_name().to_string();

                let flow_box = flow_box.clone();
                let content_stack = content_stack.clone();
                let current_items = current_items.clone();
                let image_cache = image_cache.clone();
                let nav_view = nav_view.clone();
                let player = player.clone();
                let toast_overlay = toast_overlay.clone();
                let window = window.clone();
                let client = client.clone();

                spawn_task(
                    move || plex.get_library_items(&lib_key),
                    move |result| match result {
                        Ok(items) => {
                            populate_grid(
                                &flow_box,
                                &items,
                                &current_items,
                                &image_cache,
                                &client,
                                &nav_view,
                                &player,
                                &toast_overlay,
                                &window,
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
                    },
                );
            }
        });
    }

    // ============================
    // Search handler
    // ============================

    {
        let client = client.clone();
        let flow_box = flow_box.clone();
        let content_stack = content_stack.clone();
        let current_items = current_items.clone();
        let image_cache = image_cache.clone();
        let nav_view = nav_view.clone();
        let root_page = root_page.clone();
        let player = player.clone();
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

            let c = client.borrow();
            let Some(ref plex) = *c else { return };
            let plex = plex.clone();
            drop(c);

            let flow_box = flow_box.clone();
            let content_stack = content_stack.clone();
            let current_items = current_items.clone();
            let image_cache = image_cache.clone();
            let nav_view = nav_view.clone();
            let player = player.clone();
            let toast_overlay = toast_overlay.clone();
            let window = window.clone();
            let client = client.clone();

            spawn_task(
                move || plex.search(&query),
                move |result| match result {
                    Ok(items) => {
                        populate_grid(
                            &flow_box,
                            &items,
                            &current_items,
                            &image_cache,
                            &client,
                            &nav_view,
                            &player,
                            &toast_overlay,
                            &window,
                        );
                        if items.is_empty() {
                            content_stack.set_visible_child_name("empty");
                        } else {
                            content_stack.set_visible_child_name("grid");
                        }
                    }
                    Err(e) => {
                        content_stack.set_visible_child_name("empty");
                        toast_overlay
                            .add_toast(adw::Toast::new(&format!("Search error: {}", e)));
                    }
                },
            );
        });
    }

    // ============================
    // Auto-connect if configured
    // ============================

    if config.is_configured() {
        let url = config.server_url.unwrap();
        let token = config.token.unwrap();

        let client = client.clone();
        let sidebar_list = sidebar_list.clone();
        let main_stack_c = main_stack.clone();
        let toast_overlay_c = toast_overlay.clone();

        spawn_task(
            move || match PlexClient::connect(&url, &token) {
                Ok(c) => {
                    let libs = c.get_libraries().unwrap_or_default();
                    Ok((c, libs))
                }
                Err(e) => Err(e.to_string()),
            },
            move |result: Result<(PlexClient, Vec<Library>), String>| match result {
                Ok((plex, libs)) => {
                    *client.borrow_mut() = Some(plex);
                    populate_sidebar(&sidebar_list, &libs);
                    main_stack_c.set_visible_child_name("main");
                }
                Err(_) => {
                    main_stack_c.set_visible_child_name("login");
                }
            },
        );
    } else {
        main_stack.set_visible_child_name("login");
    }

    toast_overlay.set_child(Some(&main_stack));
    window.set_content(Some(&toast_overlay));
    window.present();
}

// ============================
// Login Page
// ============================

fn build_login_page(
    client: Client,
    main_stack: gtk::Stack,
    toast_overlay: adw::ToastOverlay,
) -> gtk::Box {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_valign(gtk::Align::Center);
    outer.set_halign(gtk::Align::Center);
    outer.set_vexpand(true);

    let clamp = adw::Clamp::builder().maximum_size(420).build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 16);
    vbox.set_margin_start(24);
    vbox.set_margin_end(24);
    vbox.set_margin_top(48);
    vbox.set_margin_bottom(48);

    let icon = gtk::Image::from_icon_name("video-display-symbolic");
    icon.set_pixel_size(64);
    icon.set_margin_bottom(8);
    vbox.append(&icon);

    let title = gtk::Label::new(Some("Plex Client"));
    title.add_css_class("login-title");
    vbox.append(&title);

    let subtitle = gtk::Label::new(Some("Connect to your Plex Media Server"));
    subtitle.add_css_class("login-subtitle");
    subtitle.set_margin_bottom(16);
    vbox.append(&subtitle);

    let form_group = adw::PreferencesGroup::new();

    let url_row = adw::EntryRow::builder().title("Server URL").build();
    url_row.set_text("http://localhost:32400");
    form_group.add(&url_row);

    let token_row = adw::EntryRow::builder().title("X-Plex-Token").build();
    form_group.add(&token_row);

    vbox.append(&form_group);

    let connect_btn = gtk::Button::builder()
        .label("Connect")
        .css_classes(["suggested-action", "pill"])
        .halign(gtk::Align::Center)
        .margin_top(16)
        .build();
    vbox.append(&connect_btn);

    let help_label = gtk::Label::builder()
        .label(
            "To find your token: sign in at app.plex.tv, open browser\n\
             dev tools, find X-Plex-Token in any network request.",
        )
        .wrap(true)
        .justify(gtk::Justification::Center)
        .css_classes(["login-subtitle"])
        .margin_top(24)
        .build();
    vbox.append(&help_label);

    // Connect handler
    {
        let url_row = url_row.clone();
        let token_row = token_row.clone();

        connect_btn.connect_clicked(move |btn| {
            let url = url_row.text().to_string();
            let token = token_row.text().to_string();

            if url.trim().is_empty() || token.trim().is_empty() {
                toast_overlay.add_toast(adw::Toast::new("Please enter server URL and token"));
                return;
            }

            btn.set_sensitive(false);
            btn.set_label("Connecting...");

            let btn = btn.clone();
            let client = client.clone();
            let main_stack = main_stack.clone();
            let toast_overlay = toast_overlay.clone();
            let url_save = url.clone();
            let token_save = token.clone();

            spawn_task(
                move || match PlexClient::connect(&url, &token) {
                    Ok(c) => {
                        let libs = c.get_libraries().unwrap_or_default();
                        Ok((c, libs))
                    }
                    Err(e) => Err(e.to_string()),
                },
                move |result: Result<(PlexClient, Vec<Library>), String>| {
                    btn.set_sensitive(true);
                    btn.set_label("Connect");

                    match result {
                        Ok((plex, libs)) => {
                            let cfg = Config {
                                server_url: Some(url_save),
                                token: Some(token_save),
                                client_id: uuid::Uuid::new_v4().to_string(),
                            };
                            let _ = cfg.save();

                            // Find sidebar listbox in the main view and populate it
                            if let Some(main_view) = main_stack.child_by_name("main") {
                                find_and_populate_sidebar(&main_view, &libs);
                            }

                            *client.borrow_mut() = Some(plex);
                            main_stack.set_visible_child_name("main");
                            toast_overlay.add_toast(adw::Toast::new("Connected to Plex server"));
                        }
                        Err(e) => {
                            toast_overlay.add_toast(adw::Toast::new(&format!(
                                "Connection failed: {}",
                                e
                            )));
                        }
                    }
                },
            );
        });
    }

    clamp.set_child(Some(&vbox));
    outer.append(&clamp);
    outer
}

fn find_and_populate_sidebar(widget: &impl IsA<gtk::Widget>, libs: &[Library]) {
    let widget = widget.as_ref();
    if let Some(listbox) = widget.clone().downcast::<gtk::ListBox>().ok() {
        if listbox.has_css_class("navigation-sidebar") {
            populate_sidebar(&listbox, libs);
            return;
        }
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        find_and_populate_sidebar(&c, libs);
        child = c.next_sibling();
    }
}

// ============================
// Sidebar
// ============================

fn populate_sidebar(listbox: &gtk::ListBox, libs: &[Library]) {
    while let Some(row) = listbox.row_at_index(0) {
        listbox.remove(&row);
    }

    // Home row
    let home_label = gtk::Label::builder()
        .label("Home")
        .halign(gtk::Align::Start)
        .css_classes(["sidebar-row-label"])
        .build();
    let home_row = gtk::ListBoxRow::builder()
        .css_classes(["sidebar-row"])
        .child(&home_label)
        .build();
    home_row.set_widget_name("home");
    listbox.append(&home_row);

    let icon_for_type = |t: &str| -> &str {
        match t {
            "movie" => "\u{1f3ac}",
            "show" => "\u{1f4fa}",
            "artist" | "music" => "\u{1f3b5}",
            "photo" => "\u{1f4f7}",
            _ => "\u{1f4c1}",
        }
    };

    for lib in libs {
        let label_text = format!("{}  {}", icon_for_type(&lib.lib_type), lib.title);
        let label = gtk::Label::builder()
            .label(&label_text)
            .halign(gtk::Align::Start)
            .css_classes(["sidebar-row-label"])
            .build();
        let row = gtk::ListBoxRow::builder()
            .css_classes(["sidebar-row"])
            .child(&label)
            .build();
        row.set_widget_name(&lib.key);
        listbox.append(&row);
    }

    if let Some(first) = listbox.row_at_index(0) {
        listbox.select_row(Some(&first));
    }
}

// ============================
// Poster Grid
// ============================

fn populate_grid(
    flow_box: &gtk::FlowBox,
    items: &[MediaItem],
    current_items: &Items,
    image_cache: &ImageCache,
    client: &Client,
    nav_view: &adw::NavigationView,
    player: &Player,
    toast_overlay: &adw::ToastOverlay,
    window: &adw::ApplicationWindow,
) {
    // Clear grid
    while let Some(child) = flow_box.first_child() {
        flow_box.remove(&child);
    }

    *current_items.borrow_mut() = items.to_vec();

    let c = client.borrow();
    let Some(ref plex) = *c else { return };

    for (idx, item) in items.iter().enumerate() {
        let poster_url = item.thumb.as_ref().map(|t| plex.poster_url(t));
        let card = create_poster_card(item, poster_url, image_cache.clone());

        let card_btn = gtk::Button::builder()
            .child(&card)
            .css_classes(["flat"])
            .build();

        let current_items = current_items.clone();
        let nav_view = nav_view.clone();
        let player = player.clone();
        let client = client.clone();
        let image_cache = image_cache.clone();
        let toast_overlay = toast_overlay.clone();
        let window = window.clone();

        card_btn.connect_clicked(move |_| {
            let items = current_items.borrow();
            let Some(item) = items.get(idx) else { return };
            handle_item_click(
                item,
                &nav_view,
                &player,
                &client,
                &image_cache,
                &toast_overlay,
                &window,
            );
        });

        flow_box.insert(&card_btn, -1);
    }
}

fn create_poster_card(
    item: &MediaItem,
    poster_url: Option<String>,
    image_cache: ImageCache,
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
        .css_classes(["poster-image"])
        .build();

    if let Some(url) = poster_url {
        load_image_async(&picture, &url, image_cache);
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

fn load_image_async(picture: &gtk::Picture, url: &str, cache: ImageCache) {
    {
        let c = cache.borrow();
        if let Some(texture) = c.get(url) {
            picture.set_paintable(Some(texture));
            return;
        }
    }

    let url_owned = url.to_string();
    let picture = picture.clone();
    let cache = cache.clone();

    spawn_task(
        {
            let url = url_owned.clone();
            move || {
                reqwest::blocking::get(&url)
                    .ok()
                    .and_then(|r| r.bytes().ok())
                    .map(|b| b.to_vec())
            }
        },
        move |bytes_opt: Option<Vec<u8>>| {
            if let Some(bytes) = bytes_opt {
                if let Some(texture) = texture_from_bytes(&bytes) {
                    picture.set_paintable(Some(&texture));
                    cache.borrow_mut().insert(url_owned, texture);
                }
            }
        },
    );
}

// ============================
// Item Click Handling
// ============================

fn handle_item_click(
    item: &MediaItem,
    nav_view: &adw::NavigationView,
    player: &Player,
    client: &Client,
    image_cache: &ImageCache,
    toast_overlay: &adw::ToastOverlay,
    window: &adw::ApplicationWindow,
) {
    match item.item_type.as_deref() {
        Some("movie") | Some("episode") => {
            let detail_page = build_detail_page(item, player, client, image_cache, toast_overlay);
            nav_view.push(&detail_page);
        }
        Some("show") => {
            let Some(rk) = &item.rating_key else { return };
            let rk = rk.clone();
            let title = item.display_title();

            let c = client.borrow();
            let Some(ref plex) = *c else { return };
            let plex = plex.clone();
            drop(c);

            let nav_view = nav_view.clone();
            let player = player.clone();
            let client = client.clone();
            let image_cache = image_cache.clone();
            let toast_overlay = toast_overlay.clone();
            let window = window.clone();

            spawn_task(
                move || plex.get_children(&rk).unwrap_or_default(),
                move |seasons: Vec<MediaItem>| {
                    let page = build_seasons_page(
                        &title,
                        &seasons,
                        &nav_view,
                        &player,
                        &client,
                        &image_cache,
                        &toast_overlay,
                        &window,
                    );
                    nav_view.push(&page);
                },
            );
        }
        Some("season") => {
            let Some(rk) = &item.rating_key else { return };
            let rk = rk.clone();
            let title = item.display_title();

            let c = client.borrow();
            let Some(ref plex) = *c else { return };
            let plex = plex.clone();
            drop(c);

            let nav_view = nav_view.clone();
            let player = player.clone();
            let client = client.clone();
            let image_cache = image_cache.clone();
            let toast_overlay = toast_overlay.clone();

            spawn_task(
                move || plex.get_children(&rk).unwrap_or_default(),
                move |episodes: Vec<MediaItem>| {
                    let page = build_episodes_page(
                        &title,
                        &episodes,
                        &player,
                        &client,
                        &image_cache,
                        &toast_overlay,
                    );
                    nav_view.push(&page);
                },
            );
        }
        _ => {}
    }
}

// ============================
// Detail Page (Movie / Episode)
// ============================

fn build_detail_page(
    item: &MediaItem,
    player: &Player,
    client: &Client,
    image_cache: &ImageCache,
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

    // Poster
    let poster = gtk::Picture::builder()
        .width_request(250)
        .height_request(375)
        .css_classes(["poster-image"])
        .valign(gtk::Align::Start)
        .build();

    if let Some(thumb) = &item.thumb {
        let c = client.borrow();
        if let Some(ref plex) = *c {
            let url = plex.poster_url(thumb);
            load_image_async(&poster, &url, image_cache.clone());
        }
    }
    content.append(&poster);

    // Info column
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

    // Metadata line
    let mut meta_parts = Vec::new();
    if let Some(y) = item.year {
        meta_parts.push(y.to_string());
    }
    if let Some(cr) = &item.content_rating {
        meta_parts.push(cr.clone());
    }
    if let Some(d) = item.duration {
        meta_parts.push(format_duration(d));
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

    // Episode info
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

    // Play button
    let play_btn = gtk::Button::builder()
        .label("\u{25b6}  Play")
        .css_classes(["suggested-action", "pill"])
        .halign(gtk::Align::Start)
        .margin_top(16)
        .margin_bottom(16)
        .build();

    {
        let player = player.clone();
        let client = client.clone();
        let toast_overlay = toast_overlay.clone();
        let part_key = item.stream_part_key().map(|s| s.to_string());
        let title = item.display_title();

        play_btn.connect_clicked(move |_| {
            let Some(ref pk) = part_key else {
                toast_overlay.add_toast(adw::Toast::new("No playable media found"));
                return;
            };
            let c = client.borrow();
            let Some(ref plex) = *c else { return };
            let url = plex.stream_url(pk);
            drop(c);

            match player.borrow_mut().play(&url, &title) {
                Ok(_) => {
                    toast_overlay.add_toast(adw::Toast::new(&format!("Playing: {}", title)));
                }
                Err(e) => {
                    toast_overlay.add_toast(adw::Toast::new(&format!(
                        "Failed to launch mpv: {}. Is mpv installed?",
                        e
                    )));
                }
            }
        });
    }
    info.append(&play_btn);

    // Summary
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

    // Media info
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

// ============================
// Seasons Page
// ============================

fn build_seasons_page(
    show_title: &str,
    seasons: &[MediaItem],
    nav_view: &adw::NavigationView,
    player: &Player,
    client: &Client,
    image_cache: &ImageCache,
    toast_overlay: &adw::ToastOverlay,
    window: &adw::ApplicationWindow,
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
        let player = player.clone();
        let client = client.clone();
        let image_cache = image_cache.clone();
        let toast_overlay = toast_overlay.clone();

        row.connect_activated(move |_| {
            let Some(ref rk) = rk else { return };
            let rk = rk.clone();

            let c = client.borrow();
            let Some(ref plex) = *c else { return };
            let plex = plex.clone();
            drop(c);

            let nav_view = nav_view.clone();
            let player = player.clone();
            let client = client.clone();
            let image_cache = image_cache.clone();
            let toast_overlay = toast_overlay.clone();
            let title = title.clone();

            spawn_task(
                move || plex.get_children(&rk).unwrap_or_default(),
                move |episodes: Vec<MediaItem>| {
                    let page = build_episodes_page(
                        &title,
                        &episodes,
                        &player,
                        &client,
                        &image_cache,
                        &toast_overlay,
                    );
                    nav_view.push(&page);
                },
            );
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

// ============================
// Episodes Page
// ============================

fn build_episodes_page(
    season_title: &str,
    episodes: &[MediaItem],
    player: &Player,
    client: &Client,
    image_cache: &ImageCache,
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
            row.set_subtitle(&format_duration(d));
        }

        // Episode thumbnail
        if let Some(thumb) = &ep.thumb {
            let pic = gtk::Picture::builder()
                .width_request(120)
                .height_request(68)
                .css_classes(["poster-image"])
                .build();
            let c = client.borrow();
            if let Some(ref plex) = *c {
                let url = plex.poster_url(thumb);
                load_image_async(&pic, &url, image_cache.clone());
            }
            row.add_prefix(&pic);
        }

        let play_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
        row.add_suffix(&play_icon);

        let player = player.clone();
        let client = client.clone();
        let toast_overlay = toast_overlay.clone();
        let part_key = ep.stream_part_key().map(|s| s.to_string());
        let ep_title = ep.display_title();

        row.connect_activated(move |_| {
            let Some(ref pk) = part_key else {
                toast_overlay.add_toast(adw::Toast::new("No playable media"));
                return;
            };
            let c = client.borrow();
            let Some(ref plex) = *c else { return };
            let url = plex.stream_url(pk);
            drop(c);

            match player.borrow_mut().play(&url, &ep_title) {
                Ok(_) => {
                    toast_overlay
                        .add_toast(adw::Toast::new(&format!("Playing: {}", ep_title)));
                }
                Err(e) => {
                    toast_overlay.add_toast(adw::Toast::new(&format!("mpv error: {}", e)));
                }
            }
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

// ============================
// Helpers
// ============================

fn format_duration(ms: i64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}
