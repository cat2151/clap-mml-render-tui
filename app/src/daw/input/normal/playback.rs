use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::super::FIRST_PLAYABLE_TRACK;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::daw::input) enum NormalPlaybackShortcut {
    PreviewCurrentTrack,
    PreviewAllTracks,
    PlayFromCursor,
    TogglePlay,
}

pub(in crate::daw::input) fn normal_playback_shortcut(
    key_event: KeyEvent,
) -> Option<NormalPlaybackShortcut> {
    let shift = key_event.modifiers.contains(KeyModifiers::SHIFT);
    match key_event.code {
        KeyCode::Enter if shift => Some(NormalPlaybackShortcut::PreviewAllTracks),
        KeyCode::Char(' ') if shift => Some(NormalPlaybackShortcut::PlayFromCursor),
        KeyCode::Enter | KeyCode::Char(' ') => Some(NormalPlaybackShortcut::PreviewCurrentTrack),
        KeyCode::Char('P') => Some(NormalPlaybackShortcut::TogglePlay),
        _ => None,
    }
}

pub(in crate::daw::input) fn preview_target_tracks(
    tracks: usize,
    cursor_track: usize,
    preview_all_tracks: bool,
) -> Option<Vec<usize>> {
    if preview_all_tracks {
        return Some((FIRST_PLAYABLE_TRACK..tracks).collect());
    }
    if cursor_track < FIRST_PLAYABLE_TRACK || cursor_track >= tracks {
        return None;
    }
    Some(vec![cursor_track])
}

pub(in crate::daw::input) fn resolve_playback_start_measure_index(
    cursor_measure_index: Option<usize>,
    shortcut: NormalPlaybackShortcut,
) -> Option<usize> {
    match shortcut {
        NormalPlaybackShortcut::PlayFromCursor => cursor_measure_index,
        NormalPlaybackShortcut::PreviewCurrentTrack
        | NormalPlaybackShortcut::PreviewAllTracks
        | NormalPlaybackShortcut::TogglePlay => Some(0),
    }
}

pub(in crate::daw::input) fn format_random_patch_hot_reload_log(
    track: usize,
    displayed_measure_index: Option<usize>,
    old_effective_count: Option<usize>,
    new_effective_count: Option<usize>,
    old_measure_samples: usize,
    new_measure_samples: usize,
) -> String {
    let displayed = displayed_measure_index
        .map(|measure_index| format!("meas{}", measure_index + 1))
        .unwrap_or_else(|| "none".to_string());
    format!(
        "play: hot reload random patch track{track} display={displayed} effective_count={old_effective_count:?}->{new_effective_count:?} measure_samples={old_measure_samples}->{new_measure_samples}"
    )
}
