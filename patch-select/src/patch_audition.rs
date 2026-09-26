//! 音色 selector の試聴で「何を鳴らすか」と、それを鳴らす action への変換。
//!
//! ここは「どこを鳴らすか」（カーソル位置の音か、行全体か）を判断しない。判断は selector を
//! 持つ側が行い、結果だけを渡す。selector を持つ側の都合（入力欄のカーソル、DAW の meas）を
//! ここへ持ち込むと、使う側ごとに違う試聴の仕様が 1 か所で混ざる。

use crate::line_program::{LinePerformance, LineProgram};
use crate::notes::{CursorNotes, NoteRequest};
use crate::play_settings::PlaySettings;

/// 音色の試聴で鳴らすもの。どちらも本家に解釈させた結果で、MML 文字列ではない
/// （chord 系の行は MML を経由せずにイベント列になるため）。
#[derive(Clone, Debug, PartialEq)]
pub enum PatchAudition {
    /// 行全体。演奏設定（repeat 等）を載せて live timeline で鳴らす。
    Line(LinePerformance),
    /// 同時発音 1 つ。生 MIDI で鳴らす。
    Notes(CursorNotes),
}

/// 行を鳴らす前に音源の音色を差し替えるか。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchChange {
    /// いまの音色のまま鳴らす。
    Keep,
    /// 鳴らす前にこの音色へ差し替える（`None` は realtime server の既定音色へ戻す）。
    Switch(Option<String>),
}

/// 試聴で selector が持つ側へ求める処理。どちらも「鳴っているものを止めてから鳴らす」の意味。
#[derive(Clone, Debug, PartialEq)]
pub enum PatchAuditionAction {
    /// 音源の音色を差し替えてから、この note on を送る。`None` なら既定音色へ戻す。
    SetPatch {
        patch: Option<String>,
        notes: Option<NoteRequest>,
    },
    /// 鳴っているものを止め、あらためてこの行を頭から積む。
    PlayLine {
        patch: PatchChange,
        program: LineProgram,
    },
}

/// `audition` を `patch` で鳴らす action にする。`None` は音色を差し替えるだけ。
///
/// `current` は [`PatchChange::Keep`] のとき、1 音の経路へ渡す今の音色。
pub(crate) fn audition_action(
    audition: Option<PatchAudition>,
    patch: PatchChange,
    current: Option<&str>,
    settings: PlaySettings,
) -> PatchAuditionAction {
    let notes = match audition {
        Some(PatchAudition::Line(performance)) => {
            return PatchAuditionAction::PlayLine {
                patch,
                program: settings.program(performance),
            };
        }
        Some(PatchAudition::Notes(notes)) => {
            crate::log_line(format!(
                "action=patch-audition-note-on pitches={:?} gate_ms={}",
                notes.pitches,
                notes.duration.as_millis()
            ));
            Some(NoteRequest::from_notes(&notes))
        }
        None => None,
    };
    let patch = match patch {
        PatchChange::Keep => current.map(str::to_string),
        PatchChange::Switch(patch) => patch,
    };
    PatchAuditionAction::SetPatch { patch, notes }
}
