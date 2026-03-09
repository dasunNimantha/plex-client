mod config;
mod player;
mod plex;
mod ui;

use gtk4::glib;
use libadwaita as adw;
use adw::prelude::*;

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("dev.plexclient.app")
        .build();

    app.connect_activate(ui::build_ui);
    app.run()
}
