use gtk4 as gtk;
use gtk4::glib;
use libadwaita as adw;

use adw::prelude::*;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use crate::plex::PlexClient;
use crate::plex_auth;

use super::sidebar;
use super::state::AppState;
use super::util;

pub fn build_login_page(
    state: &AppState,
    main_stack: &gtk::Stack,
    toast_overlay: &adw::ToastOverlay,
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

    // --- Sign in with Plex button (browser OAuth) ---
    let plex_btn = gtk::Button::builder()
        .label("Sign in with Plex")
        .css_classes(["suggested-action", "pill"])
        .halign(gtk::Align::Center)
        .build();

    let status_label = gtk::Label::builder()
        .label("")
        .css_classes(["login-subtitle"])
        .visible(false)
        .build();

    let plex_spinner = gtk::Spinner::new();
    plex_spinner.set_size_request(24, 24);
    plex_spinner.set_visible(false);
    plex_spinner.set_halign(gtk::Align::Center);

    vbox.append(&plex_btn);
    vbox.append(&plex_spinner);
    vbox.append(&status_label);

    {
        let state = state.clone();
        let main_stack = main_stack.clone();
        let toast_overlay = toast_overlay.clone();
        let plex_btn = plex_btn.clone();
        let status_label = status_label.clone();
        let plex_spinner = plex_spinner.clone();

        plex_btn.clone().connect_clicked(move |_| {
            start_browser_auth(
                &state,
                &main_stack,
                &toast_overlay,
                &plex_btn,
                &status_label,
                &plex_spinner,
            );
        });
    }

    // --- Separator ---
    let sep_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    sep_box.set_halign(gtk::Align::Center);
    sep_box.set_margin_top(16);
    sep_box.set_margin_bottom(8);
    let sep_left = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep_left.set_size_request(80, -1);
    let sep_right = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep_right.set_size_request(80, -1);
    let or_label = gtk::Label::builder()
        .label("or connect manually")
        .css_classes(["login-subtitle"])
        .build();
    sep_box.append(&sep_left);
    sep_box.append(&or_label);
    sep_box.append(&sep_right);
    vbox.append(&sep_box);

    // --- Manual token entry (collapsible) ---
    let manual_revealer = gtk::Revealer::builder()
        .reveal_child(false)
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .build();

    let manual_box = gtk::Box::new(gtk::Orientation::Vertical, 12);

    let form_group = adw::PreferencesGroup::new();
    let url_row = adw::EntryRow::builder().title("Server URL").build();
    url_row.set_text("http://localhost:32400");
    form_group.add(&url_row);

    let token_row = adw::EntryRow::builder().title("X-Plex-Token").build();
    form_group.add(&token_row);
    manual_box.append(&form_group);

    let connect_btn = gtk::Button::builder()
        .label("Connect")
        .css_classes(["pill"])
        .halign(gtk::Align::Center)
        .margin_top(8)
        .build();
    manual_box.append(&connect_btn);

    manual_revealer.set_child(Some(&manual_box));
    vbox.append(&manual_revealer);

    let expand_btn = gtk::Button::builder()
        .label("Enter server URL & token")
        .css_classes(["flat"])
        .halign(gtk::Align::Center)
        .build();
    {
        let manual_revealer = manual_revealer.clone();
        let expand_btn_ref = expand_btn.clone();
        expand_btn.connect_clicked(move |_| {
            let showing = manual_revealer.reveals_child();
            manual_revealer.set_reveal_child(!showing);
            if showing {
                expand_btn_ref.set_label("Enter server URL & token");
            } else {
                expand_btn_ref.set_label("Hide manual entry");
            }
        });
    }
    vbox.append(&expand_btn);

    // --- Manual connect handler ---
    {
        let url_row = url_row.clone();
        let token_row = token_row.clone();
        let state = state.clone();
        let main_stack = main_stack.clone();
        let toast_overlay = toast_overlay.clone();

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
            let state = state.clone();
            let main_stack = main_stack.clone();
            let toast_overlay = toast_overlay.clone();
            let url_save = url.clone();
            let token_save = token.clone();
            let client_id = uuid::Uuid::new_v4().to_string();
            let client_id_save = client_id.clone();

            util::spawn_async(&state, async move {
                let c = PlexClient::connect(&url, &token, &client_id).await?;
                let libs = c.get_libraries().await.unwrap_or_default();
                Ok::<_, anyhow::Error>((c, libs))
            }, move |result, state| {
                btn.set_sensitive(true);
                btn.set_label("Connect");

                match result {
                    Ok((plex, libs)) => {
                        let mut cfg = state.config.borrow().clone();
                        cfg.server_url = Some(url_save);
                        cfg.token = Some(token_save);
                        cfg.client_id = client_id_save;
                        let _ = cfg.save();
                        *state.config.borrow_mut() = cfg;

                        if let Some(main_view) = main_stack.child_by_name("main") {
                            find_and_populate_sidebar(&main_view, &libs);
                        }

                        *state.client.borrow_mut() = Some(plex);
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
            });
        });
    }

    clamp.set_child(Some(&vbox));
    outer.append(&clamp);
    outer
}

fn start_browser_auth(
    state: &AppState,
    main_stack: &gtk::Stack,
    toast_overlay: &adw::ToastOverlay,
    plex_btn: &gtk::Button,
    status_label: &gtk::Label,
    spinner: &gtk::Spinner,
) {
    let client_id = uuid::Uuid::new_v4().to_string();

    plex_btn.set_sensitive(false);
    plex_btn.set_label("Opening browser...");
    spinner.set_visible(true);
    spinner.start();
    status_label.set_visible(true);
    status_label.set_label("Waiting for you to sign in...");

    let state = state.clone();
    let main_stack = main_stack.clone();
    let toast_overlay = toast_overlay.clone();
    let plex_btn = plex_btn.clone();
    let status_label = status_label.clone();
    let spinner = spinner.clone();
    let client_id_clone = client_id.clone();

    util::spawn_async(&state, {
        let client_id = client_id.clone();
        async move {
            plex_auth::request_pin(&client_id).await
        }
    }, move |result, state| {
        match result {
            Ok(pin) => {
                let url = plex_auth::auth_url(&client_id_clone, &pin.code);

                // Open the browser
                if let Err(e) = open_browser(&url) {
                    toast_overlay.add_toast(adw::Toast::new(&format!(
                        "Could not open browser: {}. Visit the URL manually.",
                        e
                    )));
                }

                // Start polling for auth token
                poll_for_token(
                    state,
                    &main_stack,
                    &toast_overlay,
                    &plex_btn,
                    &status_label,
                    &spinner,
                    &client_id_clone,
                    pin.id,
                    &pin.code,
                );
            }
            Err(e) => {
                reset_auth_ui(&plex_btn, &status_label, &spinner);
                toast_overlay.add_toast(adw::Toast::new(&format!("Auth error: {}", e)));
            }
        }
    });
}

fn poll_for_token(
    state: &AppState,
    main_stack: &gtk::Stack,
    toast_overlay: &adw::ToastOverlay,
    plex_btn: &gtk::Button,
    status_label: &gtk::Label,
    spinner: &gtk::Spinner,
    client_id: &str,
    pin_id: i64,
    code: &str,
) {
    let attempts = Rc::new(Cell::new(0u32));
    let done = Rc::new(Cell::new(false));
    let client_id = client_id.to_string();
    let code = code.to_string();
    let state = state.clone();
    let main_stack = main_stack.clone();
    let toast_overlay = toast_overlay.clone();
    let plex_btn = plex_btn.clone();
    let status_label = status_label.clone();
    let spinner = spinner.clone();

    glib::timeout_add_local(Duration::from_secs(2), move || {
        if done.get() {
            return glib::ControlFlow::Break;
        }

        let count = attempts.get() + 1;
        attempts.set(count);

        if count > 900 {
            reset_auth_ui(&plex_btn, &status_label, &spinner);
            toast_overlay.add_toast(adw::Toast::new("Sign in timed out. Please try again."));
            return glib::ControlFlow::Break;
        }

        let client_id = client_id.clone();
        let code = code.clone();
        let state = state.clone();
        let main_stack = main_stack.clone();
        let toast_overlay = toast_overlay.clone();
        let plex_btn = plex_btn.clone();
        let status_label = status_label.clone();
        let spinner = spinner.clone();
        let done = done.clone();

        let (tx, rx) = tokio::sync::oneshot::channel();

        state.rt.spawn({
            let client_id = client_id.clone();
            let code = code.clone();
            async move {
                let result = plex_auth::check_pin(&client_id, pin_id, &code).await;
                let _ = tx.send(result);
            }
        });

        glib::spawn_future_local(async move {
            let Ok(result) = rx.await else { return };
            match result {
                Ok(Some(token)) => {
                    done.set(true);
                    status_label.set_label("Signed in! Finding your servers...");
                    fetch_servers_and_connect(
                        &state,
                        &main_stack,
                        &toast_overlay,
                        &plex_btn,
                        &status_label,
                        &spinner,
                        &client_id,
                        &token,
                    );
                }
                Ok(None) => {}
                Err(_) => {}
            }
        });

        glib::ControlFlow::Continue
    });
}

fn fetch_servers_and_connect(
    state: &AppState,
    main_stack: &gtk::Stack,
    toast_overlay: &adw::ToastOverlay,
    plex_btn: &gtk::Button,
    status_label: &gtk::Label,
    spinner: &gtk::Spinner,
    client_id: &str,
    token: &str,
) {
    let client_id = client_id.to_string();
    let token = token.to_string();
    let state = state.clone();
    let main_stack = main_stack.clone();
    let toast_overlay = toast_overlay.clone();
    let plex_btn = plex_btn.clone();
    let status_label = status_label.clone();
    let spinner = spinner.clone();

    util::spawn_async(&state, {
        let token = token.clone();
        let client_id = client_id.clone();
        async move {
            plex_auth::get_servers(&token, &client_id).await
        }
    }, move |result, state| {
        match result {
            Ok(servers) if servers.is_empty() => {
                reset_auth_ui(&plex_btn, &status_label, &spinner);
                toast_overlay.add_toast(adw::Toast::new("No Plex servers found on your account"));
            }
            Ok(servers) if servers.len() == 1 => {
                connect_to_server(
                    state, &main_stack, &toast_overlay,
                    &plex_btn, &status_label, &spinner,
                    &servers[0], &client_id, &token,
                );
            }
            Ok(servers) => {
                show_server_picker(
                    state, &main_stack, &toast_overlay,
                    &plex_btn, &status_label, &spinner,
                    &servers, &client_id, &token,
                );
            }
            Err(e) => {
                reset_auth_ui(&plex_btn, &status_label, &spinner);
                toast_overlay.add_toast(adw::Toast::new(&format!("Failed to get servers: {}", e)));
            }
        }
    });
}

fn show_server_picker(
    state: &AppState,
    main_stack: &gtk::Stack,
    toast_overlay: &adw::ToastOverlay,
    plex_btn: &gtk::Button,
    status_label: &gtk::Label,
    spinner: &gtk::Spinner,
    servers: &[plex_auth::PlexResource],
    client_id: &str,
    token: &str,
) {
    spinner.stop();
    spinner.set_visible(false);
    plex_btn.set_visible(false);
    status_label.set_label("Choose a server:");

    let parent = status_label.parent().and_downcast::<gtk::Box>().unwrap();

    let listbox = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .margin_top(8)
        .build();
    listbox.set_widget_name("server-picker");

    for server in servers {
        let row = adw::ActionRow::builder()
            .title(&server.name)
            .activatable(true)
            .build();

        if let Some(addr) = &server.public_address {
            row.set_subtitle(addr);
        }
        row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));

        let state = state.clone();
        let main_stack = main_stack.clone();
        let toast_overlay = toast_overlay.clone();
        let plex_btn = plex_btn.clone();
        let status_label = status_label.clone();
        let spinner = spinner.clone();
        let server = server.clone();
        let client_id = client_id.to_string();
        let token = token.to_string();
        let listbox_ref = listbox.clone();

        row.connect_activated(move |_| {
            // Remove picker and connect
            if let Some(parent) = listbox_ref.parent() {
                if let Some(b) = parent.downcast_ref::<gtk::Box>() {
                    b.remove(&listbox_ref);
                }
            }
            plex_btn.set_visible(true);
            spinner.set_visible(true);
            spinner.start();
            status_label.set_label(&format!("Connecting to {}...", server.name));

            connect_to_server(
                &state, &main_stack, &toast_overlay,
                &plex_btn, &status_label, &spinner,
                &server, &client_id, &token,
            );
        });

        listbox.append(&row);
    }

    parent.append(&listbox);
}

fn connect_to_server(
    state: &AppState,
    main_stack: &gtk::Stack,
    toast_overlay: &adw::ToastOverlay,
    plex_btn: &gtk::Button,
    status_label: &gtk::Label,
    spinner: &gtk::Spinner,
    server: &plex_auth::PlexResource,
    client_id: &str,
    token: &str,
) {
    let server_token = server.access_token.clone().unwrap_or_else(|| token.to_string());
    let client_id = client_id.to_string();
    let server = server.clone();
    let state = state.clone();
    let main_stack = main_stack.clone();
    let toast_overlay = toast_overlay.clone();
    let plex_btn = plex_btn.clone();
    let status_label = status_label.clone();
    let spinner = spinner.clone();

    status_label.set_visible(true);
    status_label.set_label(&format!("Finding best connection to {}...", server.name));

    util::spawn_async(&state, {
        let server_token = server_token.clone();
        let client_id = client_id.clone();
        let server = server.clone();
        async move {
            let url = plex_auth::find_working_connection(&server, &server_token, &client_id).await?;
            let c = PlexClient::connect(&url, &server_token, &client_id).await?;
            let libs = c.get_libraries().await.unwrap_or_default();
            Ok::<_, anyhow::Error>((c, libs, url))
        }
    }, move |result, state| {
        match result {
            Ok((plex, libs, server_url)) => {
                finish_connection(state, &main_stack, &toast_overlay, &plex_btn, &status_label, &spinner, plex, libs, server_url, server_token, client_id);
            }
            Err(_) => {
                show_custom_url_entry(
                    state, &main_stack, &toast_overlay,
                    &plex_btn, &status_label, &spinner,
                    &server.name, &server_token, &client_id,
                );
            }
        }
    });
}

fn show_custom_url_entry(
    state: &AppState,
    main_stack: &gtk::Stack,
    toast_overlay: &adw::ToastOverlay,
    plex_btn: &gtk::Button,
    status_label: &gtk::Label,
    spinner: &gtk::Spinner,
    server_name: &str,
    token: &str,
    client_id: &str,
) {
    spinner.stop();
    spinner.set_visible(false);
    plex_btn.set_visible(false);
    status_label.set_visible(true);
    status_label.set_label(&format!(
        "Could not auto-detect connection to \"{}\".\nEnter your server URL (e.g. reverse proxy):",
        server_name
    ));

    let parent = status_label.parent().and_downcast::<gtk::Box>().unwrap();

    let entry_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    entry_box.set_margin_top(8);
    entry_box.set_widget_name("custom-url-box");

    let form = adw::PreferencesGroup::new();
    let url_row = adw::EntryRow::builder().title("Server URL").build();
    url_row.set_text("https://");
    form.add(&url_row);
    entry_box.append(&form);

    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    btn_box.set_halign(gtk::Align::Center);

    let connect_btn = gtk::Button::builder()
        .label("Connect")
        .css_classes(["suggested-action", "pill"])
        .build();

    let cancel_btn = gtk::Button::builder()
        .label("Cancel")
        .css_classes(["pill"])
        .build();

    btn_box.append(&cancel_btn);
    btn_box.append(&connect_btn);
    entry_box.append(&btn_box);

    parent.append(&entry_box);

    // Cancel: reset everything
    {
        let plex_btn = plex_btn.clone();
        let status_label = status_label.clone();
        let spinner = spinner.clone();
        let entry_box = entry_box.clone();
        let parent = parent.clone();

        cancel_btn.connect_clicked(move |_| {
            parent.remove(&entry_box);
            reset_auth_ui(&plex_btn, &status_label, &spinner);
        });
    }

    // Connect with custom URL
    {
        let state = state.clone();
        let main_stack = main_stack.clone();
        let toast_overlay = toast_overlay.clone();
        let plex_btn = plex_btn.clone();
        let status_label = status_label.clone();
        let spinner = spinner.clone();
        let token = token.to_string();
        let client_id = client_id.to_string();
        let entry_box_ref = entry_box.clone();
        let parent = parent.clone();

        connect_btn.connect_clicked(move |btn| {
            let url = url_row.text().to_string();
            if url.trim().is_empty() || url.trim() == "https://" || url.trim() == "http://" {
                toast_overlay.add_toast(adw::Toast::new("Please enter your server URL"));
                return;
            }

            btn.set_sensitive(false);
            btn.set_label("Connecting...");
            status_label.set_label("Connecting...");
            spinner.set_visible(true);
            spinner.start();

            let btn = btn.clone();
            let state = state.clone();
            let main_stack = main_stack.clone();
            let toast_overlay = toast_overlay.clone();
            let plex_btn = plex_btn.clone();
            let status_label = status_label.clone();
            let spinner = spinner.clone();
            let token = token.clone();
            let client_id = client_id.clone();
            let entry_box_ref = entry_box_ref.clone();
            let parent = parent.clone();

            util::spawn_async(&state, {
                let url = url.clone();
                let token = token.clone();
                let client_id = client_id.clone();
                async move {
                    let c = PlexClient::connect(&url, &token, &client_id).await?;
                    let libs = c.get_libraries().await.unwrap_or_default();
                    Ok::<_, anyhow::Error>((c, libs, url))
                }
            }, move |result, state| {
                btn.set_sensitive(true);
                btn.set_label("Connect");

                match result {
                    Ok((plex, libs, server_url)) => {
                        parent.remove(&entry_box_ref);
                        finish_connection(state, &main_stack, &toast_overlay, &plex_btn, &status_label, &spinner, plex, libs, server_url, token, client_id);
                    }
                    Err(e) => {
                        spinner.stop();
                        spinner.set_visible(false);
                        status_label.set_label("Connection failed. Check the URL and try again.");
                        toast_overlay.add_toast(adw::Toast::new(&format!("Error: {}", e)));
                    }
                }
            });
        });
    }
}

fn finish_connection(
    state: &AppState,
    main_stack: &gtk::Stack,
    toast_overlay: &adw::ToastOverlay,
    plex_btn: &gtk::Button,
    status_label: &gtk::Label,
    spinner: &gtk::Spinner,
    plex: PlexClient,
    libs: Vec<crate::plex::Library>,
    server_url: String,
    token: String,
    client_id: String,
) {
    let mut cfg = state.config.borrow().clone();
    cfg.server_url = Some(server_url);
    cfg.token = Some(token);
    cfg.client_id = client_id;
    let _ = cfg.save();
    *state.config.borrow_mut() = cfg;

    if let Some(main_view) = main_stack.child_by_name("main") {
        find_and_populate_sidebar(&main_view, &libs);
    }

    *state.client.borrow_mut() = Some(plex);
    main_stack.set_visible_child_name("main");
    reset_auth_ui(plex_btn, status_label, spinner);
    toast_overlay.add_toast(adw::Toast::new("Connected to Plex server"));
}

fn reset_auth_ui(btn: &gtk::Button, status: &gtk::Label, spinner: &gtk::Spinner) {
    btn.set_sensitive(true);
    btn.set_label("Sign in with Plex");
    btn.set_visible(true);
    spinner.stop();
    spinner.set_visible(false);
    status.set_visible(false);
    status.set_label("");
}

fn open_browser(url: &str) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn find_and_populate_sidebar(widget: &impl gtk::prelude::IsA<gtk::Widget>, libs: &[crate::plex::Library]) {
    let widget = widget.as_ref();
    if let Ok(listbox) = widget.clone().downcast::<gtk::ListBox>() {
        if listbox.has_css_class("navigation-sidebar") {
            sidebar::populate_sidebar(&listbox, libs);
            return;
        }
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        find_and_populate_sidebar(&c, libs);
        child = c.next_sibling();
    }
}
