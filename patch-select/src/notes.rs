//! 同時発音 1 つを鳴らすときに運ぶ型。
//!
//! どの音を鳴らすかを MML から求める関数はここに無い（MML を解釈する側が持つ）。

use std::ops::Range;
use std::time::Duration;

use crate::NOTE_ON;

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

/// 生 MIDI の note on と、送信成功後に保つべき音長。
#[derive(Clone, Debug, PartialEq)]
pub struct NoteRequest {
    pub messages: Vec<[u8; 3]>,
    pub duration: Duration,
}

impl NoteRequest {
    /// 同時発音の note on。前の音を止めるのは受け取る側の仕事。
    pub fn from_notes(notes: &CursorNotes) -> Self {
        Self {
            messages: notes
                .pitches
                .iter()
                .map(|pitch| [NOTE_ON, *pitch, notes.velocity])
                .collect(),
            duration: notes.duration,
        }
    }
}
