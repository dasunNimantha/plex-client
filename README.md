# Plex Client

A native Plex media client for Linux, built with Rust + GTK4/libadwaita + mpv.

No Snap. No Flatpak. Just a native binary.

## Features

- Browse your Plex libraries (Movies, TV Shows, Music)
- View On Deck and Recently Added
- Search across your library
- Full TV show navigation (Show → Seasons → Episodes)
- Media detail view with poster, metadata, and summary
- Playback via mpv with IPC control
- Automatic playback progress reporting to Plex (syncs "Continue Watching")
- Saves server configuration for quick reconnect

## Dependencies

### Build dependencies

**Ubuntu / Debian:**

```bash
sudo apt install build-essential libgtk-4-dev libadwaita-1-dev libgdk-pixbuf-2.0-dev pkg-config
```

**Fedora / RHEL / CentOS:**

```bash
sudo dnf install gcc gtk4-devel libadwaita-devel gdk-pixbuf2-devel pkg-config
```

**Arch / Manjaro:**

```bash
sudo pacman -S base-devel gtk4 libadwaita
```

**openSUSE:**

```bash
sudo zypper install gcc gtk4-devel libadwaita-devel gdk-pixbuf-devel pkg-config
```

**Alpine:**

```bash
sudo apk add build-base gtk4.0-dev libadwaita-dev gdk-pixbuf-dev pkgconf
```

**Void Linux:**

```bash
sudo xbps-install -S base-devel gtk4-devel libadwaita-devel gdk-pixbuf-devel pkg-config
```

**Nix / NixOS:**

```bash
nix-shell -p pkg-config gtk4 libadwaita gdk-pixbuf
```

You also need Rust installed. If you don't have it:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Runtime dependencies

mpv is required for media playback:

| Distro | Command |
|---|---|
| Ubuntu / Debian | `sudo apt install mpv` |
| Fedora | `sudo dnf install mpv` |
| Arch / Manjaro | `sudo pacman -S mpv` |
| openSUSE | `sudo zypper install mpv` |
| Alpine | `sudo apk add mpv` |
| Void | `sudo xbps-install -S mpv` |
| Nix | `nix-env -iA nixpkgs.mpv` |

## Building

```bash
cargo build --release
```

The binary will be at `target/release/plex-client`.

## Running

```bash
cargo run --release
```

On first launch, enter your Plex server URL and token to connect. Configuration is saved to `~/.config/plex-client/config.json`.

## Getting Your Plex Token

1. Sign in at [app.plex.tv](https://app.plex.tv)
2. Open browser developer tools (F12)
3. Go to the Network tab
4. Look for `X-Plex-Token` in any request's query parameters or headers
5. Copy that token value into the client

## Architecture

```
src/
├── main.rs          Entry point, tokio runtime + GTK app
├── config.rs        Server config persistence
├── plex.rs          Plex REST API client (async)
├── player.rs        mpv subprocess + IPC control
└── ui/
    ├── mod.rs       Main window layout and wiring
    ├── state.rs     Shared application state
    ├── style.rs     CSS theming
    ├── util.rs      Async helpers, image loading, formatting
    ├── login.rs     Server connection page
    ├── sidebar.rs   Library navigation sidebar
    ├── grid.rs      Poster grid and item click handling
    ├── detail.rs    Movie/episode detail view
    ├── seasons.rs   TV show seasons list
    ├── episodes.rs  Season episodes list
    └── playback.rs  Progress tracking and Plex timeline reporting
```

- **GTK4 + libadwaita** — Native Linux desktop UI with Adwaita styling
- **reqwest + rustls** — Async HTTP client with pure-Rust TLS (no system OpenSSL needed)
- **tokio** — Async runtime for non-blocking API calls and mpv IPC
- **mpv** — External media player with IPC socket for playback state monitoring
- **serde** — JSON parsing for Plex API responses

## Keyboard Shortcuts

All mpv keyboard shortcuts work during playback (space for pause, arrow keys for seeking, etc.)
