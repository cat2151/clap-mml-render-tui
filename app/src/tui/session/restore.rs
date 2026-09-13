//! 保存済みセッション（`cmrt-history` の素の値）を、画面が持つ型へ起こす。
//!
//! 逆向き（画面 → history）は [`super::persist`]。
//! 対応表をこの2モジュールだけが知っている状態を保つこと。

/// 復元したセッションを、画面ごとの型へ配る前の中間表現。
pub(super) struct LoadedSessionState {
    pub(super) cursor: usize,
    pub(super) lines: Vec<String>,
    pub(super) active_screen: crate::screen_switch::PrimaryScreen,
    pub(super) keyboard: crate::history::KeyboardSessionState,
    pub(super) grid_sequencer_track_count: usize,
    pub(super) grid_sequencer_chord_mode: bool,
    pub(super) grid_sequencer: Option<crate::history::GridSequencerSessionState>,
    pub(super) grid_sequencer_bpm: Option<f64>,
    pub(super) loop_browser_bpm: Option<f64>,
    pub(super) grid_sequencer_bpm_range: Option<[f64; 2]>,
    pub(super) loop_browser_bpm_range: Option<[f64; 2]>,
    pub(super) keyboard_note_guide_overlay_date: Option<String>,
    pub(super) notepad_sound_check_guide_overlay_date: Option<String>,
    pub(super) mml_overlay_patch: Option<String>,
    pub(super) chord_chart_patch: Option<String>,
    pub(super) mml_overlay_play_settings: crate::history::MmlOverlayPlaySettings,
}

pub(super) fn load_initial_session_state() -> LoadedSessionState {
    // `lines` は常に1行以上を保持する（不変条件）。
    // load_session_state() は lines が空でないことを保証している。
    let crate::history::SessionState {
        cursor,
        lines,
        active_screen,
        keyboard,
        grid_sequencer_track_count,
        grid_sequencer_chord_mode,
        grid_sequencer,
        grid_sequencer_bpm,
        loop_browser_bpm,
        grid_sequencer_bpm_range,
        loop_browser_bpm_range,
        keyboard_note_guide_overlay_date,
        notepad_sound_check_guide_overlay_date,
        mml_overlay_patch,
        chord_chart_patch,
        mml_overlay_play_settings,
    } = crate::history::load_session_state();
    let initial_cursor = super::clamp_session_cursor(cursor, lines.len());
    LoadedSessionState {
        cursor: initial_cursor,
        lines,
        active_screen,
        keyboard,
        grid_sequencer_track_count,
        grid_sequencer_chord_mode,
        grid_sequencer,
        grid_sequencer_bpm,
        loop_browser_bpm,
        grid_sequencer_bpm_range,
        loop_browser_bpm_range,
        keyboard_note_guide_overlay_date,
        notepad_sound_check_guide_overlay_date,
        mml_overlay_patch,
        chord_chart_patch,
        mml_overlay_play_settings,
    }
}

/// 保存済みの自動BPM範囲を復元する。未保存・不正なら `default_bpm` 固定の範囲。
pub(super) fn bpm_range_from_history(
    saved: Option<[f64; 2]>,
    default_bpm: f64,
) -> cmrt_tui_core::bpm::BpmRange {
    saved
        .and_then(|[minimum, maximum]| cmrt_tui_core::bpm::BpmRange::new(minimum, maximum))
        .unwrap_or_else(|| cmrt_tui_core::bpm::BpmRange::fixed(default_bpm))
}

/// 復元直後の BPM モード。手動値が残っていればそれを、無ければ範囲から1つ引く。
pub(super) fn restored_bpm_mode(
    manual: Option<f64>,
    range: cmrt_tui_core::bpm::BpmRange,
) -> cmrt_tui_core::bpm::BpmMode {
    cmrt_tui_core::bpm::BpmMode::from_saved(manual, range.sample())
}

/// MML overlay の演奏設定を history の素の bool から起こす。
///
/// `cmrt-history` は overlay に依存しない（依存させると依存方向が逆流する）ので、
/// 型の詰め替えは両方を知っているここが行う。
pub(super) fn play_settings_from_history(
    saved: crate::history::MmlOverlayPlaySettings,
) -> crate::tui::mml_overlay::PlaySettings {
    crate::tui::mml_overlay::PlaySettings {
        repeat: saved.repeat,
        filters: crate::tui::mml_overlay::line_play::FilterSettings {
            modulation: saved.modulation,
            velocity: saved.velocity,
        },
    }
}
