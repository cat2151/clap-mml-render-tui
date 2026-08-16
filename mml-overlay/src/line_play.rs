//! 入力欄の 1 行を、そのまま演奏できるイベント列へ変換する。
//!
//! 行は独立して解釈する。前の行のオクターブや velocity は引き継がない（聴き比べたい
//! フレーズを 1 行ずつ書き並べる画面なので、上の行の状態が下へ漏れると困る）。
//!
//! パースの前段に chord2mml を挟む。コード表記として解釈できたらコードとして、
//! できなければこれまでどおり MML として鳴らす。どちらだったかは入力欄の下に出す。

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

/// 1 行を演奏用のイベント列へ変換する。
///
/// 空のイベント列は「走っている演奏を止めるだけ」の意味になる。空行やエラー行へ
/// カーソルを移したときも、前の行が鳴り続けないようにこれを返す。
pub fn line_events(line: &str) -> (LineStatus, Vec<TimedMidiEvent>) {
    if line.trim().is_empty() {
        return (LineStatus::Idle, Vec::new());
    }
    match cmrt_chord::timed_performance(line) {
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
                performance.events,
            )
        }
        Err(error) => (LineStatus::Error(error), Vec::new()),
    }
}

#[cfg(test)]
mod tests;
