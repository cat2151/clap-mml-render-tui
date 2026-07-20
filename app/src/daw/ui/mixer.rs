use ratatui::{layout::Rect, Frame};

use super::super::{DawApp, FIRST_PLAYABLE_TRACK};
use crate::mixer_overlay::{draw_mixer_overlay, MixerOverlayTrack};

pub(super) fn draw_mixer(frame: &mut Frame<'_>, app: &DawApp, area: Rect) {
    let tracks = (FIRST_PLAYABLE_TRACK..app.tracks)
        .map(|track| MixerOverlayTrack {
            label: format!("track{track}"),
            volume_db: app.track_volume_db(track),
        })
        .collect::<Vec<_>>();
    draw_mixer_overlay(
        frame,
        area,
        &tracks,
        app.mixer_cursor_track.saturating_sub(FIRST_PLAYABLE_TRACK),
    );
}
