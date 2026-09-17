//! patch selector のテスト。overlay の開閉・3 pane の移動・Regex 絞り込み・popup 上の
//! mouse・PATCH 欄の wheel 送りで分けてある。

use std::time::Instant;

use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::patch_load::PatchLoadState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::*;
use crate::{patch_notice::PatchUnavailable, tests::ctx_with, ChordPlayback, GridRow};

const AREA: Rect = Rect::new(0, 0, 100, 30);

/// PATCH 欄の列。grid は中央寄せなので、chord 行の有無で左端が動く。
fn patch_column(screen: &GridSequencerScreen) -> u16 {
    crate::ui::layout_for(screen, AREA).patch_column()
}

fn patches() -> PatchLoadState {
    PatchLoadState::ready(
        [
            "Bass/Mono.fxp",
            "Keys/Alpha.fxp",
            "Keys/Beta.fxp",
            "Keys/Unknown.fxp",
            "Pads/Poly.fxp",
        ]
        .into_iter()
        .map(|patch| (patch.to_string(), patch.to_lowercase()))
        .collect(),
    )
}

struct Voicing;

impl crate::GridVoicingLookup for Voicing {
    fn cached_voicing(&self, patch: &str) -> Option<PatchVoicing> {
        match patch {
            "Keys/Alpha.fxp" | "Keys/Beta.fxp" | "Pads/Poly.fxp" | "Pads/Pad 01.fxp" => {
                Some(PatchVoicing::Poly)
            }
            "Bass/Mono.fxp" => Some(PatchVoicing::Mono),
            "Keys/Unknown.fxp" => Some(PatchVoicing::Unknown),
            _ => None,
        }
    }
}

fn context(patch_load: &PatchLoadState) -> GridSequencerContext<'_> {
    ctx_with(patch_load, crate::tests::empty_catalog(), &Voicing)
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

/// 開けなかった／回せなかった理由。無反応と区別するため、必ず残っていること。
fn notice_reason(screen: &GridSequencerScreen) -> PatchUnavailable {
    screen
        .patch_notice
        .as_ref()
        .expect("開けなかった理由が通知に残る")
        .reason
        .clone()
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn selector(screen: &GridSequencerScreen) -> &PatchSelector {
    screen.patch_selector.as_ref().expect("selector is open")
}

/// 今見えている一覧の表示名。
fn filtered_patches(selector: &PatchSelector) -> Vec<&str> {
    (0..selector.filtered_len())
        .filter_map(|index| selector.filtered_entry(index))
        .map(cmrt_mml_overlay::PatchCatalogEntry::display)
        .collect()
}

/// builtin preset の語を含む名前の一覧。行用途ごとの Role / Preset を確かめる用。
fn role_patches() -> PatchLoadState {
    PatchLoadState::ready(
        [
            "Basses/Bass 01.fxp",
            "Basses/Bass 02.fxp",
            "Drums/Hi Hat 01.wav",
            "Drums/Kick 01.wav",
            "Drums/Perc Clap 01.wav",
            "Drums/Snare 01.wav",
            "Leads/Lead 01.fxp",
            "Pads/Pad 01.fxp",
        ]
        .into_iter()
        .map(|patch| (patch.to_string(), patch.to_lowercase()))
        .collect(),
    )
}

/// `/` で開く Regex 絞り込み。
mod filter;
/// popup 上の click / wheel。
mod mouse;
/// 3 pane の focus / cursor 移動と、行用途で決まる開いたときの位置。
mod navigation;
/// overlay を開いてから選ぶまで。
mod overlay;
/// 音色を差し替えた瞬間に、鳴っていた音を鳴らし直す。
mod reattack;
/// PATCH 欄の wheel で patch list を送る側（[`crate::patch_bag`]）。
mod wheel;
