use ratatui::{layout::Rect, Frame};

use super::super::{DawApp, FIRST_PLAYABLE_TRACK};
use super::patch_display;
use cmrt_tui_core::mixer_overlay::{draw_mixer_overlay, MixerOverlayTrack};

pub(super) fn draw_mixer(frame: &mut Frame<'_>, app: &DawApp, area: Rect) {
    // role 表示に使う catalog。1 描画につき 1 回だけ lock を取る（track ごとに取らない）。
    let catalog = app.catalog_snapshot();
    let tracks = (FIRST_PLAYABLE_TRACK..app.editor.tracks)
        .map(|track| {
            let init_cell = app.editor.data[track]
                .first()
                .map(String::as_str)
                .unwrap_or_default();
            let display = patch_display::track_patch_display(init_cell, catalog.as_deref());
            MixerOverlayTrack {
                // grid の行頭ラベルと同じ綴り。`track1` のように別の綴りにすると、
                // 画面をまたいで同じ track を指していることが読み取れない。
                label: crate::tracks::track_label(track),
                volume_db: app.track_volume_db(track),
                role: Some(display.role_label()),
                patch: Some(display.patch_label()),
            }
        })
        .collect::<Vec<_>>();
    draw_mixer_overlay(
        frame,
        area,
        &tracks,
        app.overlays
            .mixer
            .cursor_track
            .saturating_sub(FIRST_PLAYABLE_TRACK),
    );
}
