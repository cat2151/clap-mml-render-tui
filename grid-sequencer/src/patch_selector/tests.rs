//! patch selector のテスト。overlay の開閉と、PATCH 欄の wheel 送りで分けてある。

use std::time::Instant;

use cmrt_realtime_play::PatchVoicing;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::*;
use crate::{tests::ctx_with, ChordPlayback, GridPatchLoad, GridRow, PatternEvolution};

const AREA: Rect = Rect::new(0, 0, 100, 30);

/// PATCH 欄の列。grid は中央寄せなので、chord 行の有無で左端が動く。
fn patch_column(screen: &GridSequencerScreen) -> u16 {
    crate::ui::layout_for(screen, AREA).patch_column()
}

fn patches() -> Vec<(String, String)> {
    [
        "Bass/Mono.fxp",
        "Keys/Alpha.fxp",
        "Keys/Beta.fxp",
        "Keys/Unknown.fxp",
        "Pads/Poly.fxp",
    ]
    .into_iter()
    .map(|patch| (patch.to_string(), patch.to_lowercase()))
    .collect()
}

struct Voicing;

impl crate::GridVoicingLookup for Voicing {
    fn cached_voicing(&self, patch: &str) -> Option<PatchVoicing> {
        match patch {
            "Keys/Alpha.fxp" | "Keys/Beta.fxp" | "Pads/Poly.fxp" => Some(PatchVoicing::Poly),
            "Bass/Mono.fxp" => Some(PatchVoicing::Mono),
            "Keys/Unknown.fxp" => Some(PatchVoicing::Unknown),
            _ => None,
        }
    }
}

fn context(patches: &[(String, String)]) -> GridSequencerContext<'_> {
    ctx_with(
        GridPatchLoad::Ready(patches),
        crate::tests::empty_catalog(),
        &Voicing,
    )
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
/// overlay を開いてから選ぶまで。
mod overlay;
/// PATCH 欄の wheel で patch list を送る側（[`crate::patch_bag`]）。
mod wheel;
