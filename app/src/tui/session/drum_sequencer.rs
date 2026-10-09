//! Drum Sequencer の画面状態と `cmrt-history` の DTO の詰め替え。
//! 入力は DTO に入れず、kit ごとの pattern ファイルから読む。

use cmrt_drum_sequencer::DrumSequencerScreen;

use crate::history::DrumSequencerSessionState;
use crate::tui::drum_sequencer_glue::{load_kit_patterns, DrumSequencerState};

/// kit を選んでいなければ保存しない（history.json にキーを増やさない）。
pub(super) fn drum_session_to_history(
    screen: &DrumSequencerScreen,
) -> Option<DrumSequencerSessionState> {
    Some(DrumSequencerSessionState {
        kit: Some(screen.kit_name()?.to_string()),
        pattern: screen.pattern_index(),
        cursor_note: screen.saved_cursor_note(),
        cursor_step: screen.cursor_step(),
    })
}

/// kit の pattern はすぐ読む。note 一覧と構成音名は持たないので、catalog が揃った後に host が照合する。
pub(super) fn drum_session_from_history<'a>(
    saved: Option<DrumSequencerSessionState>,
) -> DrumSequencerState<'a> {
    let mut screen = DrumSequencerScreen::default();
    let Some(saved) = saved else {
        return DrumSequencerState::restored(screen, None);
    };
    let (patterns, error) = saved
        .kit
        .as_deref()
        .map(load_kit_patterns)
        .unwrap_or_default();
    screen.restore(
        saved.kit,
        patterns,
        saved.pattern,
        saved.cursor_note,
        saved.cursor_step,
    );
    DrumSequencerState::restored(screen, error)
}
