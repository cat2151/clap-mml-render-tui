//! chord mode の進行を grid のすぐ上へ1行で出す。
//!
//! 視線は grid 本体にあるので、進行は画面下部のステータス行ではなくここへ置く。
//! chord mode が off で、開始できなかった理由も無いときは行そのものを出さない
//! （その1行を grid に使う）。

use ratatui::text::{Line, Span};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK},
};

use crate::{ChordPlayback, GridSequencerScreen};

/// 進行のコードどうしの区切り。カタログの記法（ハイフン区切り）に合わせる。
const DEGREE_SEPARATOR: &str = "-";

/// 出すものが無ければ `None`。呼び出し側はそのぶん行を確保しない。
pub(super) fn line(screen: &GridSequencerScreen) -> Option<Line<'static>> {
    if let Some(error) = screen.chord_error() {
        return Some(Line::from(Span::styled(
            format!(" chord: {error} "),
            base_style().fg(MONOKAI_PINK),
        )));
    }
    screen.state.chord().map(progression_line)
}

fn progression_line(chord: &ChordPlayback) -> Line<'static> {
    let dim = base_style().fg(MONOKAI_GRAY);
    let mut spans = vec![Span::styled(format!(" chord Key:{}  ", chord.key()), dim)];
    spans.extend(degree_spans(chord));
    spans.push(Span::styled(
        format!("  [{}/{}] ", chord.index() + 1, chord.chord_count()),
        dim,
    ));
    Line::from(spans)
}

/// 進行をコードごとに分け、いま鳴っているコードだけ明るくする。
///
/// 分割数が進行のコード数と食い違ったら（カタログの記法が変わった場合など）、
/// 強調をあきらめて素の文字列をそのまま出す。位置を偽るよりは何も強調しないほうがよい。
fn degree_spans(chord: &ChordPlayback) -> Vec<Span<'static>> {
    let dim = base_style().fg(MONOKAI_GRAY);
    let current = base_style().fg(MONOKAI_GREEN);
    let degrees = chord.degrees().split(DEGREE_SEPARATOR).collect::<Vec<_>>();
    if degrees.len() != chord.chord_count() {
        return vec![Span::styled(chord.degrees().to_string(), dim)];
    }
    let mut spans = Vec::new();
    for (index, degree) in degrees.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(DEGREE_SEPARATOR, dim));
        }
        let style = if index == chord.index() { current } else { dim };
        spans.push(Span::styled((*degree).to_string(), style));
    }
    spans
}

#[cfg(test)]
mod tests;
