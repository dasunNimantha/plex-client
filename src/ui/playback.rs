use gtk4::glib;
use std::time::Duration;

use super::state::AppState;

/// Polls the embedded player's state every 5 seconds and reports progress to Plex.
/// Stops automatically when the player stops or goes idle.
pub fn start_progress_tracking(
    state: &AppState,
    rating_key: Option<String>,
    duration_ms: Option<i64>,
) {
    let Some(rk) = rating_key else { return };
    let dur = duration_ms.unwrap_or(0);

    let state = state.clone();

    glib::timeout_add_local(Duration::from_secs(5), move || {
        if !state.player_widget.is_playing() {
            report_to_plex(&state, &rk, 0, dur, "stopped");
            return glib::ControlFlow::Break;
        }

        let offset_ms = state.player_widget.get_position_ms();
        let plex_state = if state.player_widget.get_paused() {
            "paused"
        } else {
            "playing"
        };
        report_to_plex(&state, &rk, offset_ms, dur, plex_state);

        glib::ControlFlow::Continue
    });
}

fn report_to_plex(
    state: &AppState,
    rating_key: &str,
    offset_ms: i64,
    duration_ms: i64,
    plex_state: &str,
) {
    let plex = {
        let c = state.client.borrow();
        match c.as_ref() {
            Some(p) => p.clone(),
            None => return,
        }
    };

    let rk = rating_key.to_string();
    let ps = plex_state.to_string();

    state.rt.spawn(async move {
        let _ = plex.report_progress(&rk, offset_ms, &ps, duration_ms).await;
    });
}
