# Plex Client

A native Plex media client for Linux, built with Rust + GTK4/libadwaita + mpv.

## Features

- Browse your Plex libraries (Movies, TV Shows, Music)
- View On Deck and Recently Added
- Search across your library
- Full TV show navigation (Show → Seasons → Episodes)
- Media detail view with poster, metadata, and summary
- Playback via mpv (best codec support on Linux)
- Saves server configuration for quick reconnect

## Dependencies

### Build dependencies (Manjaro/Arch)

```bash
sudo pacman -S gtk4 libadwaita base-devel rust
```

### Runtime dependencies

```bash
sudo pacman -S mpv
```

## Building

```bash
cargo build --release
```

The binary will be at `target/release/plex-client`.

## Running

```bash
cargo run --release
```

On first launch, enter your Plex server URL and token to connect.

## Getting Your Plex Token

1. Sign in at [app.plex.tv](https://app.plex.tv)
2. Open browser developer tools (F12)
3. Go to the Network tab
4. Look for `X-Plex-Token` in any request's query parameters or headers
5. Copy that token value into the client

## Architecture

- **GTK4 + libadwaita** — Native Linux desktop UI
- **reqwest** — HTTP client for Plex API
- **mpv** — External media player launched with stream URLs, controlled via IPC socket
- **serde** — JSON parsing for Plex API responses

## Keyboard Shortcuts

All mpv keyboard shortcuts work during playback (space for pause, arrow keys for seeking, etc.)
