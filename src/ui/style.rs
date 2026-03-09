use gtk4 as gtk;
use gtk4::gdk;

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
.playback-bar {
    background-color: alpha(@window_bg_color, 0.95);
    padding: 8px 16px;
    border-top: 1px solid alpha(currentColor, 0.1);
}
.playback-title {
    font-size: 13px;
    font-weight: 500;
}
.playback-time {
    font-size: 11px;
    opacity: 0.6;
    font-family: monospace;
}
"#;

pub fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
