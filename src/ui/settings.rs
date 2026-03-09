use gtk4 as gtk;
use libadwaita as adw;

use adw::prelude::*;

use crate::config;

use super::state::AppState;

pub fn build_settings_page(state: &AppState) -> adw::NavigationPage {
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();

    let clamp = adw::Clamp::builder()
        .maximum_size(600)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 16);

    // --- Hardware Decoding ---
    let hwdec_group = adw::PreferencesGroup::builder()
        .title("Hardware Video Decoding")
        .description("Select the GPU decoder for video playback. Restart the app to apply.")
        .build();

    let available = config::detect_available_hwdec();
    let current_hwdec = state.config.borrow().hwdec.clone();

    let check_group: Vec<gtk::CheckButton> = Vec::new();
    let mut first_check: Option<gtk::CheckButton> = None;

    for entry in &available {
        let row = adw::ActionRow::builder()
            .title(entry.label.as_str())
            .activatable(true)
            .build();

        let check = gtk::CheckButton::new();
        if entry.mode == current_hwdec {
            check.set_active(true);
        }
        if let Some(ref first) = first_check {
            check.set_group(Some(first));
        } else {
            first_check = Some(check.clone());
        }

        let mode = entry.mode.clone();
        let state_c = state.clone();
        check.connect_toggled(move |btn| {
            if btn.is_active() {
                let mut cfg = state_c.config.borrow_mut();
                cfg.hwdec = mode.clone();
                let _ = cfg.save();
            }
        });

        let check_for_row = check.clone();
        row.connect_activated(move |_| {
            check_for_row.set_active(true);
        });

        row.add_prefix(&check);
        hwdec_group.add(&row);
    }

    let _ = check_group;

    vbox.append(&hwdec_group);

    // --- Playback ---
    let playback_group = adw::PreferencesGroup::builder()
        .title("Playback")
        .build();

    let seek_row = adw::ActionRow::builder()
        .title("Seek step (seconds)")
        .subtitle("Time to skip with left/right arrow keys")
        .build();

    let current_seek = state.config.borrow().seek_seconds;
    let seek_spin = gtk::SpinButton::with_range(1.0, 60.0, 1.0);
    seek_spin.set_value(current_seek as f64);
    seek_spin.set_valign(gtk::Align::Center);

    {
        let state_c = state.clone();
        seek_spin.connect_value_changed(move |btn| {
            let val = btn.value() as u32;
            let mut cfg = state_c.config.borrow_mut();
            cfg.seek_seconds = val;
            let _ = cfg.save();
            drop(cfg);
            state_c.player_widget.set_seek_seconds(val);
        });
    }

    seek_row.add_suffix(&seek_spin);
    playback_group.add(&seek_row);
    vbox.append(&playback_group);

    // --- About ---
    let about_group = adw::PreferencesGroup::builder()
        .title("About")
        .build();

    let version_row = adw::ActionRow::builder()
        .title("Version")
        .subtitle(env!("CARGO_PKG_VERSION"))
        .build();
    about_group.add(&version_row);

    let logout_btn = gtk::Button::builder()
        .label("Sign Out")
        .css_classes(["destructive-action", "pill"])
        .halign(gtk::Align::Center)
        .margin_top(24)
        .build();

    {
        let state = state.clone();
        logout_btn.connect_clicked(move |_| {
            let mut cfg = state.config.borrow_mut();
            cfg.server_url = None;
            cfg.token = None;
            let _ = cfg.save();
            drop(cfg);
            *state.client.borrow_mut() = None;
            state.main_stack.set_visible_child_name("login");
        });
    }

    vbox.append(&about_group);
    vbox.append(&logout_btn);

    clamp.set_child(Some(&vbox));
    scroll.set_child(Some(&clamp));
    toolbar.set_content(Some(&scroll));

    adw::NavigationPage::builder()
        .title("Settings")
        .child(&toolbar)
        .build()
}
