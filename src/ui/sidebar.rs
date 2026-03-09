use gtk4 as gtk;
use gtk::prelude::*;

use crate::plex::Library;

pub fn build_sidebar() -> (gtk::Box, gtk::ListBox, gtk::Button) {
    let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar_box.set_size_request(240, -1);
    sidebar_box.add_css_class("sidebar");

    let sidebar_header = libadwaita::HeaderBar::builder()
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

    let settings_btn = gtk::Button::builder()
        .label("Settings")
        .icon_name("emblem-system-symbolic")
        .css_classes(["flat"])
        .margin_start(8)
        .margin_end(8)
        .margin_top(4)
        .margin_bottom(8)
        .build();
    sidebar_box.append(&settings_btn);

    (sidebar_box, sidebar_list, settings_btn)
}

pub fn populate_sidebar(listbox: &gtk::ListBox, libs: &[Library]) {
    while let Some(child) = listbox.first_child() {
        listbox.remove(&child);
    }

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
