mod config;
mod plex;
mod plex_auth;
mod ui;

use gtk4::glib;
use libadwaita as adw;
use adw::prelude::*;

fn main() -> glib::ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let _guard = rt.enter();

    let app = adw::Application::builder()
        .application_id("dev.plexclient.app")
        .build();

    app.connect_activate(move |app| {
        ui::build_ui(app, rt.handle().clone());
    });

    app.run()
}
