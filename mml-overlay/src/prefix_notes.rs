//! カーソル位置までの MML を解釈して「いま鳴らすべき音」を求める。
//!
//! MML はカーソルより前の内容（オクターブ・velocity）で解釈が変わるため、打鍵の
//! たびにカーソル位置までの prefix を丸ごと本家パーサへ通す。こうすると
//! `c+1<c+1` のような修飾の解釈が必ず本家と一致し、こちら側に MML の解釈は
//! 一切持たなくて済む。

use mmlabc_to_smf::{pass1_parser::parse_mml, pass2_ast::tokens_to_ast, types::AstNote};

/// `AstNote::note_type` のうち、実際に発音するもの。
/// `rest` / `program_change` / `tempo_set` も同じ列に混ざって来るため必ず絞る。
const NOTE_TYPE: &str = "note";

/// velocity 未指定時の既定値（本家 pass2 の既定 v15 = 127 と揃える）。
const DEFAULT_VELOCITY: u8 = 127;

/// prefix を解釈した結果の「いま鳴らすべき同時発音」。
///
/// 発音するかどうかは「前回の値と違うか」だけで決める。カーソルを左へ戻して
/// 同じ音高へ帰ってきたときに鳴らし直せるよう、音高だけでなく prefix 内での
/// 通し番号も同一性に含める。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefixNotes {
    /// prefix 内で何番目のノートか（0 始まり）。
    pub note_index: usize,
    /// 末尾の同時発音。単音なら 1 要素、和音 `'ceg'` なら複数。
    pub pitches: Vec<u8>,
    pub velocity: u8,
}

/// 文字単位のカーソル位置をバイト位置へ直す。
///
/// `TextArea::cursor()` が返す列は文字単位なので、そのまま文字列の添字には使えない。
pub fn prefix_byte_index(text: &str, cursor_chars: usize) -> usize {
    text.char_indices()
        .nth(cursor_chars)
        .map_or(text.len(), |(index, _)| index)
}

/// カーソル位置までの MML から、いま鳴らすべき音を求める。
pub fn notes_at_cursor(text: &str, cursor_chars: usize) -> Option<PrefixNotes> {
    notes_at_prefix(&text[..prefix_byte_index(text, cursor_chars)])
}

/// prefix 文字列から、いま鳴らすべき音を求める。
///
/// prefix は入力途中なので構文としては未完成になり得るが、tree-sitter は
/// エラー耐性があるため解釈できたところまでが返る。
pub fn notes_at_prefix(prefix: &str) -> Option<PrefixNotes> {
    if prefix.is_empty() {
        return None;
    }
    let ast = tokens_to_ast(&parse_mml(prefix));
    let notes = ast
        .notes
        .iter()
        .filter(|note| note.note_type == NOTE_TYPE)
        .collect::<Vec<_>>();
    let last = *notes.last()?;
    Some(PrefixNotes {
        note_index: notes.len() - 1,
        pitches: trailing_simultaneous(&notes, last),
        velocity: last.velocity.unwrap_or(DEFAULT_VELOCITY),
    })
}

/// 末尾ノートと同時に鳴るノート群を、末尾から遡って集める。
///
/// 和音 `'ceg'` の構成音は同じ `chord_id` を共有する。`chord_id` を持たない
/// 単音のときは末尾の 1 音だけが対象。
fn trailing_simultaneous(notes: &[&AstNote], last: &AstNote) -> Vec<u8> {
    let Some(chord_id) = last.chord_id else {
        return vec![last.pitch];
    };
    let mut pitches = notes
        .iter()
        .rev()
        .take_while(|note| note.chord_id == Some(chord_id))
        .map(|note| note.pitch)
        .collect::<Vec<_>>();
    pitches.reverse();
    pitches
}

#[cfg(test)]
mod tests;
