//! カーソルのある発音単位を求め、それを鳴らすための音を返す。
//!
//! どこからどこまでが 1 つの発音単位かは、本家 mmlabc-to-smf の CST から
//! [`cmrt_chord::cursor_sounding_unit`] 経由で受け取る。こちらは MML の文法を
//! ひとつも持たない。
//!
//! 音高・velocity・音長は、その単位の終わりまでの MML を本家 SMF へ通して
//! 末尾の note on を読むだけで求める。テンポやオクターブや `l` はカーソルより前の
//! 内容で変わるので、単位だけを切り出して鳴らすことはできない。
//!
//! カーソルの位置ではなく**単位の終わり**で切るのが要点。`c4` の途中に
//! カーソルがあっても 4 分音符として鳴り、和音 `'ceg'` の中にいれば
//! 構成音ぜんぶが鳴る。
//!
//! 切り出しは必ず行の内側で行う。オーバーレイは行志向で、前の行のオクターブや
//! velocity を引き継がないため。

use std::ops::Range;
use std::time::Duration;

use cmrt_chord::{cursor_sounding_unit, timed_performance, TimedMidiEvent};

use crate::{NOTE_OFF, NOTE_ON};

/// MML がまだ空のときに、音色の試聴だけを目的に鳴らす音。
///
/// 音高も velocity も音長も、既定値のまま鳴らした 1 音がほしいだけなので
/// MML で書いて本家に解かせる。
pub(crate) const PREVIEW_MML: &str = "c";

/// カーソルのある発音単位と、そこで鳴らすべき同時発音。
///
/// 発音するかどうかは「前回と違うか」だけで決める。[`CursorNotes::span`] を
/// 同一性に含めるので、同じ単位の内側でカーソルが動くあいだは鳴らし直さず、
/// 別の単位へ移れば同じ音高でも鳴り直す。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorNotes {
    /// 行の中での発音単位の範囲。
    pub span: Range<usize>,
    /// 同時発音。単音なら 1 要素、和音 `'ceg'` なら複数。
    pub pitches: Vec<u8>,
    pub velocity: u8,
    /// 単位が chord 表記として解釈されたか。表示だけに使う。
    pub from_chord: bool,
    /// 書かれたとおりの音長。本家 SMF の note on から note off までの時間。
    pub duration: Duration,
}

/// 文字単位のカーソル位置をバイト位置へ直す。
///
/// `TextArea::cursor()` が返す列は文字単位なので、そのまま文字列の添字には使えない。
pub fn cursor_byte_index(text: &str, cursor_chars: usize) -> usize {
    text.char_indices()
        .nth(cursor_chars)
        .map_or(text.len(), |(index, _)| index)
}

/// カーソルのある発音単位を鳴らすための音を求める。
///
/// 休符やコマンドの上、どの単位にも触れていない空白の上では `None`。
pub fn notes_at_cursor(line: &str, cursor_chars: usize) -> Option<CursorNotes> {
    let span = cursor_sounding_unit(line, cursor_byte_index(line, cursor_chars))?;
    notes_of(&line[..span.end], span)
}

/// MML の末尾の音を、prefix ぜんぶを 1 単位として求める。
pub(crate) fn notes_at_prefix(prefix: &str) -> Option<CursorNotes> {
    notes_of(prefix, 0..prefix.len())
}

/// `mml` を本家へ通し、末尾の同時発音を `span` の単位の音として返す。
///
/// `mml` は入力途中なので構文としては未完成になり得るが、tree-sitter は
/// エラー耐性があるため解釈できたところまでが返る。発音するノートが 1 つも
/// 無ければ `None`。
fn notes_of(mml: &str, span: Range<usize>) -> Option<CursorNotes> {
    let performance = timed_performance(mml).ok()?;
    let events = &performance.events;
    let onset = events.iter().rposition(is_note_on)?;
    let pitches = simultaneous_pitches(events, onset);
    Some(CursorNotes {
        span,
        velocity: events[onset].message[2],
        duration: sounding_duration(events, onset, &pitches),
        pitches,
        from_chord: performance.from_chord,
    })
}

fn is_note_on(event: &TimedMidiEvent) -> bool {
    event.message[0] == NOTE_ON
}

/// 末尾ノートと同時に鳴るノート群。和音 `'ceg'` の構成音は同じ時刻に並ぶ。
fn simultaneous_pitches(events: &[TimedMidiEvent], onset: usize) -> Vec<u8> {
    let seconds = events[onset].seconds;
    let mut pitches = events[..=onset]
        .iter()
        .rev()
        .take_while(|event| is_note_on(event) && event.seconds == seconds)
        .map(|event| event.message[1])
        .collect::<Vec<_>>();
    pitches.reverse();
    pitches
}

/// note on から note off までの時間。和音は最初に切れる構成音に合わせる。
///
/// 打鍵で鳴らした音はこちらが note off を送って止めるので、本家 SMF が書いた
/// 長さをそのまま gate の長さに使う。
fn sounding_duration(events: &[TimedMidiEvent], onset: usize, pitches: &[u8]) -> Duration {
    let seconds = pitches
        .iter()
        .filter_map(|pitch| note_off_seconds(&events[onset + 1..], *pitch))
        .fold(f64::INFINITY, f64::min)
        - events[onset].seconds;
    // note off が見つからない MML は無いはずだが、見つからなければ差が
    // 無限大や負になる。そのときは鳴らしっぱなしにせず即座に切る。
    Duration::try_from_secs_f64(seconds).unwrap_or(Duration::ZERO)
}

fn note_off_seconds(events: &[TimedMidiEvent], pitch: u8) -> Option<f64> {
    events
        .iter()
        .find(|event| event.message[0] == NOTE_OFF && event.message[1] == pitch)
        .map(|event| event.seconds)
}

#[cfg(test)]
mod tests;
