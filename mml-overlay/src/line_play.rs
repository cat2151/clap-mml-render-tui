//! 入力欄の 1 行を、そのまま演奏できるイベント列へ変換する。
//!
//! 行は独立して解釈する。前の行のオクターブや velocity は引き継がない（聴き比べたい
//! フレーズを 1 行ずつ書き並べる画面なので、上の行の状態が下へ漏れると困る）。
//!
//! パースの前段に chord2mml を挟む。コード表記として解釈できたらコードとして、
//! できなければこれまでどおり MML として鳴らす。どちらだったかは入力欄の下に出す。
//!
//! 「行をどう鳴らすか」（1 回だけか鳴らし続けるか、MIDI filter を重ねるか）も
//! [`LineProgram`] としてここに置く。イベント列と鳴らし方は必ず対で運ばれ、
//! 受け取る [`crate::sender`] が両方を見て初めて 1 回の演奏になるため。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::NOTE_ON;
use cmrt_chord::TimedMidiEvent;

/// 直近に行を演奏した結果。入力欄の下に出す。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum LineStatus {
    /// まだ演奏していない、または空行にいる。
    #[default]
    Idle,
    Played {
        /// chord2mml がコード表記として受け付けたか。
        from_chord: bool,
        /// 鳴らすノート数（和音は構成音ぶん数える）。
        note_count: usize,
    },
    Error(String),
}

/// 1 行ぶんの演奏内容。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinePerformance {
    /// 時刻順に並んだ note on / note off。空なら「止めるだけ」の意味になる。
    pub events: Vec<TimedMidiEvent>,
    /// 1 周の長さ。繰り返すとき、次の周をこれだけ後ろへずらす。
    ///
    /// **罠: これは `cmrt_chord::TimedPerformance::duration_seconds` そのままで、
    /// 「最後のイベントまで」しか測っていない。行末の休符は落ちる。**
    /// ループが詰まって聞こえたらここが原因。
    pub loop_seconds: f64,
}

impl LinePerformance {
    /// 鳴らすものが無い。受け取った側は前の演奏を止めるだけになる。
    pub fn silent() -> Self {
        Self::default()
    }

    pub fn is_silent(&self) -> bool {
        self.events.is_empty()
    }
}

/// 演奏へ重ねる MIDI filter の ON/OFF。
///
/// 実際に掛けるのは [`crate::sender`] 側。ここは「掛けてほしいか」だけを運ぶ。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FilterSettings {
    /// CC1 modulation を LFO で重ねる。
    pub modulation: bool,
    /// note on の velocity を LFO の値で乗っ取る（MML の `v` 指定を無視する）。
    pub velocity: bool,
}

/// 「この行をどう鳴らしてほしいか」ひとまとまり。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineProgram {
    pub performance: LinePerformance,
    /// 鳴らし終わっても止めず、同じ内容を継ぎ足して鳴らし続ける。
    pub repeat: bool,
    pub filters: FilterSettings,
}

impl LineProgram {
    /// 1 回だけ鳴らす（filter なし）。演奏設定がまだ無い経路はこれを使う。
    pub fn once(performance: LinePerformance) -> Self {
        Self {
            performance,
            repeat: false,
            filters: FilterSettings::default(),
        }
    }

    /// 鳴らすものが無い。前の演奏を止めるだけの指示になる。
    pub fn silent() -> Self {
        Self::once(LinePerformance::silent())
    }

    pub fn is_silent(&self) -> bool {
        self.performance.is_silent()
    }

    pub fn events(&self) -> &[TimedMidiEvent] {
        &self.performance.events
    }
}

/// 1 行を演奏用のイベント列へ変換する。
///
/// 空のイベント列は「走っている演奏を止めるだけ」の意味になる。空行やエラー行へ
/// カーソルを移したときも、前の行が鳴り続けないようにこれを返す。
pub fn line_events(line: &str) -> (LineStatus, LinePerformance) {
    if line.trim().is_empty() {
        return (LineStatus::Idle, LinePerformance::silent());
    }
    performance_events(cmrt_chord::timed_performance(line))
}

/// chord 行の 1 セルを、本番と同じ init/directive 文脈で演奏用イベント列へ変換する。
pub fn chord_line_events(
    line: &str,
    chord_init: &str,
    track_directive: &str,
    mml_prefix: &str,
) -> (LineStatus, LinePerformance) {
    if line.trim().is_empty() {
        return (LineStatus::Idle, LinePerformance::silent());
    }
    performance_events(cmrt_chord::timed_chord_cell_performance(
        chord_init,
        track_directive,
        mml_prefix,
        line,
    ))
}

/// Chord Chart の section 進行を、Key 文脈つきでそのまま演奏用イベント列へ変換する。
///
/// DAW chord cell の外側 bar は追加せず、変換失敗時も MML へ fallback しない。
pub fn chord_chart_line_events(
    line: &str,
    key_token: Option<&str>,
) -> (LineStatus, LinePerformance) {
    if line.trim().is_empty() {
        return (LineStatus::Idle, LinePerformance::silent());
    }
    performance_events(cmrt_chord::timed_chord_progression_performance(
        key_token, line,
    ))
}

/// Chord Chart の進行を auto voicing 済みの音高で鳴らす。
///
/// `voicings` は section 単独または Arrangement 全体から呼び出し側が選んだもの。
/// `chord_index` があれば、全体内で決まった転回形を保ったままその chord だけを返す。
pub fn auto_voiced_chord_chart_line_events(
    line: &str,
    key_token: Option<&str>,
    voicings: &[cmrt_chord::ChordVoicing],
    chord_index: Option<usize>,
) -> (LineStatus, LinePerformance) {
    if line.trim().is_empty() {
        return (LineStatus::Idle, LinePerformance::silent());
    }
    let performance =
        cmrt_chord::timed_chord_progression_performance(key_token, line).and_then(|performance| {
            cmrt_chord::revoice_timed_progression(performance, voicings, chord_index)
        });
    performance_events(performance)
}

/// Chord Chart の section 単独で auto voicing した行を鳴らす。
pub fn locally_auto_voiced_chord_chart_line_events(
    line: &str,
    key_token: Option<&str>,
    chord_index: Option<usize>,
) -> (LineStatus, LinePerformance) {
    if line.trim().is_empty() {
        return (LineStatus::Idle, LinePerformance::silent());
    }
    performance_events(cmrt_chord::timed_auto_voiced_chord_progression_performance(
        key_token,
        line,
        chord_index,
    ))
}

/// Chord Chart の section をローカルに auto voice し、Bass パートだけを鳴らす。
pub fn locally_auto_voiced_bass_chord_chart_line_events(
    line: &str,
    key_token: Option<&str>,
    chord_index: Option<usize>,
) -> (LineStatus, LinePerformance) {
    if line.trim().is_empty() {
        return (LineStatus::Idle, LinePerformance::silent());
    }
    performance_events(
        cmrt_chord::timed_auto_voiced_bass_chord_progression_performance(
            key_token,
            line,
            chord_index,
        ),
    )
}

fn performance_events(
    result: Result<cmrt_chord::TimedPerformance, String>,
) -> (LineStatus, LinePerformance) {
    match result {
        Ok(performance) => {
            let note_count = performance
                .events
                .iter()
                .filter(|event| event.message[0] == NOTE_ON)
                .count();
            (
                LineStatus::Played {
                    from_chord: performance.from_chord,
                    note_count,
                },
                LinePerformance {
                    events: performance.events,
                    loop_seconds: performance.duration_seconds,
                },
            )
        }
        Err(error) => (LineStatus::Error(error), LinePerformance::silent()),
    }
}

/// このキーはカーソルのある行をもう一度鳴らす。
///
/// 行が変わったときは自動で鳴るが、同じ行を鳴らし直す手段が別に要る。
/// `Ctrl+Space` は端末によって `Char(' ')` と `Char('\0')` のどちらでも届く。
///
/// 音色選択（`Ctrl+T`）を開いている間は `Space` が試聴キーになるため、この判定は
/// 通常の入力欄だけで使う。
pub(crate) fn is_replay_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(' ') | KeyCode::Char('\0'))
}

#[cfg(test)]
mod tests;
