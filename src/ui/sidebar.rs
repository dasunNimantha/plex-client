use gtk4 as gtk;
use gtk::prelude::*;

use crate::plex::Library;

pub fn build_sidebar() -> (gtk::Box, gtk::ListBox, gtk::Button) {
    let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar_box.set_size_request(200, -1);
    sidebar_box.add_css_class("sidebar");

    let sidebar_header = libadwaita::HeaderBar::builder()
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();
    sidebar_header.add_css_class("sidebar-header");

    let header_label = gtk::Label::new(None);
    header_label.set_markup("<span font_weight='800' font_size='large' foreground='#E5A00D'>PLEX</span>");
    sidebar_header.set_title_widget(Some(&header_label));
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

    let settings_btn = gtk::Button::builder()
        .icon_name("emblem-system-symbolic")
        .label("Settings")
        .css_classes(["flat", "sidebar-settings-btn"])
        .build();
    sidebar_box.append(&settings_btn);

    (sidebar_box, sidebar_list, settings_btn)
}

fn icon_for_type(lib_type: &str) -> &str {
    match lib_type {
        "movie" => "camera-video-symbolic",
        "show" => "video-display-symbolic",
        "artist" | "music" => "audio-headphones-symbolic",
        "photo" => "camera-photo-symbolic",
        _ => "folder-symbolic",
    }
}

fn build_sidebar_row(label_text: &str, icon_name: &str) -> gtk::ListBoxRow {
    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    hbox.set_margin_start(4);
    hbox.set_margin_end(4);

    let icon = gtk::Image::from_icon_name(icon_name);
    icon.add_css_class("sidebar-icon");
    icon.set_pixel_size(16);
    hbox.append(&icon);

    let label = gtk::Label::builder()
        .label(label_text)
        .halign(gtk::Align::Start)
        .hexpand(true)
        .css_classes(["sidebar-row-label"])
        .build();
    hbox.append(&label);

    gtk::ListBoxRow::builder()
        .css_classes(["sidebar-row"])
        .child(&hbox)
        .build()
}

pub fn populate_sidebar(listbox: &gtk::ListBox, libs: &[Library]) {
    while let Some(child) = listbox.first_child() {
        listbox.remove(&child);
    }

    let home_row = build_sidebar_row("Home", "go-home-symbolic");
    home_row.set_widget_name("home");
    listbox.append(&home_row);

    if !libs.is_empty() {
        let section_label = gtk::Label::builder()
            .label("LIBRARIES")
            .halign(gtk::Align::Start)
            .css_classes(["sidebar-section-label"])
            .build();
        let section_row = gtk::ListBoxRow::builder()
            .selectable(false)
            .activatable(false)
            .child(&section_label)
            .build();
        listbox.append(&section_row);
    }

    for lib in libs {
        let icon = icon_for_type(&lib.lib_type);
        let row = build_sidebar_row(&lib.title, icon);
        row.set_widget_name(&lib.key);
        listbox.append(&row);
    }

    if let Some(first) = listbox.row_at_index(0) {
        listbox.select_row(Some(&first));
    }
}
