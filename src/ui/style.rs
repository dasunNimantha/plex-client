use gtk4 as gtk;
use gtk4::gdk;
use libadwaita as adw;

const CSS: &str = r#"

/* ── Global ─────────────────────────────────────────────── */
window {
    background-color: #282828;
    color: #e8e8e8;
}
headerbar {
    background-color: #1F1F1F;
    border-bottom: 1px solid rgba(255,255,255,0.06);
    color: #e0e0e0;
    min-height: 38px;
}
headerbar .title {
    font-weight: 600;
    font-size: 13px;
}
.navigation-sidebar {
    background-color: transparent;
}

/* ── Sidebar ────────────────────────────────────────────── */
.sidebar {
    background-color: #1F1F1F;
    border-right: 1px solid rgba(255,255,255,0.06);
}
.sidebar-header {
    background-color: transparent;
    border-bottom: 1px solid rgba(255,255,255,0.06);
    min-height: 38px;
}
.sidebar-section-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: rgba(255,255,255,0.30);
    padding: 12px 20px 4px 20px;
}
.sidebar-row {
    padding: 7px 12px;
    margin: 1px 6px;
    border-radius: 4px;
    transition: background-color 120ms ease;
    color: rgba(255,255,255,0.60);
    min-height: 28px;
}
.sidebar-row:hover {
    background-color: rgba(255,255,255,0.05);
    color: rgba(255,255,255,0.85);
}
.sidebar-row:selected {
    background-color: rgba(229,160,13,0.12);
    color: #E5A00D;
}
.sidebar-icon {
    margin-right: 10px;
    color: inherit;
}
.sidebar-row-label {
    font-weight: 500;
    font-size: 13px;
    color: inherit;
}
.sidebar-settings-btn {
    margin: 4px 8px 8px 8px;
    border-radius: 4px;
    color: rgba(255,255,255,0.45);
    background-color: transparent;
    border: 1px solid rgba(255,255,255,0.06);
    font-size: 12px;
}
.sidebar-settings-btn:hover {
    background-color: rgba(255,255,255,0.05);
    color: rgba(255,255,255,0.8);
}

/* ── Poster Cards (portrait) ────────────────────────────── */
.poster-card {
    padding: 4px;
    border-radius: 6px;
    transition: background-color 150ms ease;
}
.poster-card:hover {
    background-color: rgba(255,255,255,0.06);
}
.poster-image {
    border-radius: 4px;
    background-color: rgba(255,255,255,0.04);
}
.poster-title {
    font-size: 13px;
    font-weight: 500;
    margin-top: 6px;
    color: rgba(255,255,255,0.85);
}
.poster-subtitle {
    font-size: 11px;
    color: rgba(255,255,255,0.40);
    margin-top: 1px;
}
.poster-progress-track {
    background-color: rgba(255,255,255,0.10);
    border-radius: 0 0 4px 4px;
    min-height: 3px;
    margin-top: 0;
}
.poster-progress-fill {
    background-color: #E5A00D;
    border-radius: 0 0 0 4px;
    min-height: 3px;
}

/* ── Landscape Cards (continue watching) ────────────────── */
.landscape-card {
    padding: 4px;
    border-radius: 6px;
    transition: background-color 150ms ease;
}
.landscape-card:hover {
    background-color: rgba(255,255,255,0.06);
}
.landscape-image {
    border-radius: 4px;
    background-color: rgba(255,255,255,0.04);
}
.landscape-title {
    font-size: 13px;
    font-weight: 500;
    margin-top: 5px;
    color: rgba(255,255,255,0.85);
}
.landscape-info {
    font-size: 11px;
    color: rgba(255,255,255,0.40);
    margin-top: 1px;
}
.landscape-remaining {
    font-size: 11px;
    font-weight: 600;
    color: rgba(255,255,255,0.55);
}

/* ── Hub / Shelf ────────────────────────────────────────── */
.hub-section {
    margin-bottom: 2px;
}
.hub-title {
    font-size: 20px;
    font-weight: 700;
    color: rgba(255,255,255,0.90);
    margin: 20px 24px 10px 24px;
}
.hub-divider {
    background-color: rgba(255,255,255,0.06);
    min-height: 1px;
    margin: 8px 24px 0 24px;
}

/* ── Detail Page ────────────────────────────────────────── */
.detail-page-bg {
    background-color: #282828;
}
.detail-backdrop {
    opacity: 0.55;
    border-radius: 0;
}
.detail-gradient {
    background: linear-gradient(to top, #282828 0%, rgba(40,40,40,0.85) 35%, rgba(40,40,40,0.3) 70%, transparent 100%);
}
.detail-title {
    font-size: 28px;
    font-weight: 800;
    color: #FFFFFF;
}
.detail-meta {
    font-size: 13px;
    color: rgba(255,255,255,0.50);
}
.detail-meta-tag {
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 3px;
    border: 1px solid rgba(255,255,255,0.30);
    color: rgba(255,255,255,0.60);
}
.detail-summary {
    font-size: 14px;
    color: rgba(255,255,255,0.65);
}
.detail-media-info {
    font-size: 12px;
    color: rgba(255,255,255,0.30);
    font-family: monospace;
}
.detail-poster {
    border-radius: 6px;
    background-color: rgba(255,255,255,0.04);
}
.plex-play-btn {
    background-color: #E5A00D;
    color: #1a1a1a;
    font-size: 14px;
    font-weight: 700;
    padding: 8px 28px;
    border-radius: 4px;
    border: none;
}
.plex-play-btn:hover {
    background-color: #f0b020;
}
.plex-play-btn:active {
    background-color: #cc8e0b;
}

/* ── Player Controls ────────────────────────────────────── */
.playback-bar {
    background-color: rgba(10, 10, 10, 0.88);
    padding: 8px 20px;
    border-radius: 10px;
    color: white;
}
.playback-bar button {
    color: rgba(255,255,255,0.85);
}
.playback-bar button:hover {
    color: white;
    background-color: rgba(255,255,255,0.1);
}
.playback-title {
    font-size: 13px;
    font-weight: 500;
    color: rgba(255,255,255,0.75);
}
.playback-time {
    font-size: 11px;
    color: rgba(255,255,255,0.55);
    font-family: monospace;
}
scale trough {
    background-color: rgba(255,255,255,0.15);
    border-radius: 2px;
    min-height: 4px;
}
scale trough highlight {
    background-color: #E5A00D;
    border-radius: 2px;
    min-height: 4px;
}
scale slider {
    background-color: #E5A00D;
    border: 2px solid white;
    min-width: 14px;
    min-height: 14px;
    border-radius: 50%;
    margin: -5px 0;
}

/* ── Login Page ─────────────────────────────────────────── */
.login-page {
    background-color: #1F1F1F;
}
.login-title {
    font-size: 26px;
    font-weight: 800;
    color: white;
}
.login-subtitle {
    font-size: 13px;
    color: rgba(255,255,255,0.45);
}
.plex-sign-in-btn {
    background-color: #E5A00D;
    color: #1a1a1a;
    font-weight: 700;
    font-size: 14px;
    padding: 10px 36px;
    border-radius: 4px;
    border: none;
}
.plex-sign-in-btn:hover {
    background-color: #f0b020;
}

/* ── Loading / Spinner ──────────────────────────────────── */
.loading-label {
    color: rgba(255,255,255,0.40);
    font-size: 13px;
}
spinner {
    color: #E5A00D;
}

/* ── Content area ───────────────────────────────────────── */
.content-bg {
    background-color: #282828;
}
.empty-state-icon {
    color: rgba(255,255,255,0.10);
}

/* ── Search ─────────────────────────────────────────────── */
searchentry {
    background-color: rgba(255,255,255,0.06);
    color: rgba(255,255,255,0.85);
    border: 1px solid rgba(255,255,255,0.06);
    border-radius: 4px;
    font-size: 13px;
}
searchentry:focus {
    border-color: rgba(229,160,13,0.4);
    background-color: rgba(255,255,255,0.08);
}

/* ── Episode/Season Lists ───────────────────────────────── */
.ep-list {
    background-color: transparent;
    border: none;
    border-radius: 0;
}
.ep-row {
    background-color: rgba(255,255,255,0.025);
    border-radius: 6px;
    margin-bottom: 4px;
    transition: background-color 120ms ease;
}
.ep-row:hover {
    background-color: rgba(255,255,255,0.06);
}
.ep-num {
    font-size: 14px;
    font-weight: 600;
    color: rgba(255,255,255,0.30);
    min-width: 28px;
}
.ep-title {
    font-size: 14px;
    font-weight: 500;
    color: rgba(255,255,255,0.85);
}
.ep-duration {
    font-size: 12px;
    color: rgba(255,255,255,0.35);
}
.ep-desc {
    font-size: 12px;
    color: rgba(255,255,255,0.35);
    margin-top: 2px;
}
.ep-play-icon {
    color: rgba(255,255,255,0.30);
}
.ep-play-icon:hover {
    color: #E5A00D;
}

.boxed-list {
    background-color: rgba(255,255,255,0.025);
    border: 1px solid rgba(255,255,255,0.06);
    border-radius: 8px;
}
row {
    color: rgba(255,255,255,0.85);
}
row:hover {
    background-color: rgba(255,255,255,0.04);
}

/* ── Settings ───────────────────────────────────────────── */
.preferences-group {
    color: rgba(255,255,255,0.9);
}
checkbutton indicator {
    border-color: rgba(255,255,255,0.2);
}
checkbutton:checked indicator {
    background-color: #E5A00D;
    border-color: #E5A00D;
}

/* ── Section Title ──────────────────────────────────────── */
.section-title {
    font-size: 20px;
    font-weight: bold;
    margin-top: 16px;
    margin-bottom: 8px;
    color: rgba(255,255,255,0.9);
}

/* ── Toast ──────────────────────────────────────────────── */
toast {
    background-color: rgba(229,160,13,0.9);
    color: #1a1a1a;
    font-weight: 600;
}

"#;

pub fn load_css() {
    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);

    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
