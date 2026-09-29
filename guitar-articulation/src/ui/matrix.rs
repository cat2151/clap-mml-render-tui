//! 音とルールの matrix。横 = 列（同時刻の note on のまとまり）、縦 = 使われている音高（高い音が上）。
//! 音の段の下に、行全体のルールの段と、列ごとのルールの段を並べる。

use std::collections::BTreeSet;

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK},
};

use super::{note_name, ROW_RULE_ROWS, RULE_ROWS};
use crate::{picks_string, strings_by_column, Articulation, GuitarArticulationScreen, RowRule};

/// 段の見出しの桁（`F#3` や `a:H/P`）。
const LABEL_WIDTH: usize = 6;
/// 1 列の桁（記号 1 つ + 空白）。
const COLUMN_WIDTH: usize = 2;
const NOTE_MARK: &str = "■";
const NO_NOTE_MARK: &str = "·";
const RULE_ON_MARK: &str = "●";
/// 自動ハンマリングが ON で、ピッキングのまま残す列。
const AUTO_PICK_MARK: &str = "p";
const DOWN_STROKE_MARK: &str = "D";
const UP_STROKE_MARK: &str = "U";
/// エコノミーピッキングで、ピッキングしない（H/P の）頂点の音。
const LEGATO_ACCENT_MARK: &str = "^";
const EMPTY_TEXT: &str = "(MML を入れると音が並びます)";

/// 枠の内側に要る行数。使われている音高の段と、ルールの段。
pub(super) fn height(screen: &GuitarArticulationScreen) -> u16 {
    match used_pitches(screen).len() {
        0 => 1,
        pitches => (pitches + ROW_RULE_ROWS.len() + RULE_ROWS.len()) as u16,
    }
}

/// 使われている音高（低い順）。音の無い音高は段を作らない。
fn used_pitches(screen: &GuitarArticulationScreen) -> BTreeSet<u8> {
    screen.notes().iter().map(|note| note.pitch).collect()
}

pub(super) fn lines(screen: &GuitarArticulationScreen, width: u16) -> Vec<Line<'static>> {
    let pitches = used_pitches(screen);
    if pitches.is_empty() {
        return vec![Line::styled(EMPTY_TEXT, base_style().fg(MONOKAI_GRAY))];
    }
    let columns = visible_columns(screen, width);
    let mut out = Vec::new();
    for &pitch in pitches.iter().rev() {
        let cells = columns.clone().map(|column| {
            let sounding = screen
                .notes()
                .iter()
                .any(|note| note.column == column && note.pitch == pitch);
            if sounding {
                (NOTE_MARK, base_style().fg(MONOKAI_CYAN))
            } else {
                (NO_NOTE_MARK, base_style().fg(MONOKAI_GRAY))
            }
        });
        out.push(row(
            &note_name(pitch),
            cells,
            screen.cursor(),
            columns.start,
        ));
    }
    let strings = strings_by_column(screen.notes());
    for (rule, key, name) in ROW_RULE_ROWS {
        let on = screen.rules().is_row_on(rule);
        let cells = columns.clone().map(|column| match (on, rule) {
            (false, _) => (" ", base_style()),
            (true, RowRule::AutoHammerPull) => auto_hammer_pull_cell(&strings, column),
            (true, RowRule::EconomyPicking) => economy_picking_cell(screen, column),
        });
        out.push(row(
            &format!("{key}:{name}"),
            cells,
            screen.cursor(),
            columns.start,
        ));
    }
    for (rule, key, name) in RULE_ROWS {
        let cells = columns.clone().map(|column| {
            if screen.rules().is_on(column, rule) {
                (RULE_ON_MARK, base_style().fg(MONOKAI_GREEN))
            } else {
                (" ", base_style())
            }
        });
        out.push(row(
            &format!("{key}:{name}"),
            cells,
            screen.cursor(),
            columns.start,
        ));
    }
    out
}

fn auto_hammer_pull_cell(strings: &[usize], column: usize) -> (&'static str, Style) {
    if picks_string(strings, column) {
        (AUTO_PICK_MARK, base_style().fg(MONOKAI_GRAY))
    } else {
        (RULE_ON_MARK, base_style().fg(MONOKAI_GREEN))
    }
}

/// ストローク（D/U）。頂点のアクセントはピンクの太字。和音の列は先頭の音で代表する。
fn economy_picking_cell(screen: &GuitarArticulationScreen, column: usize) -> (&'static str, Style) {
    let Some(articulated) = screen
        .notes()
        .iter()
        .position(|note| note.column == column)
        .and_then(|i| screen.articulated().get(i))
    else {
        return (" ", base_style());
    };
    let mark = match (articulated.articulation, articulated.accent) {
        (Articulation::SusDown, _) => DOWN_STROKE_MARK,
        (Articulation::SusUp, _) => UP_STROKE_MARK,
        (_, true) => LEGATO_ACCENT_MARK,
        (_, false) => " ",
    };
    let style = if articulated.accent {
        base_style().fg(MONOKAI_PINK).add_modifier(Modifier::BOLD)
    } else {
        base_style().fg(MONOKAI_GREEN)
    };
    (mark, style)
}

/// カーソル列が必ず見えるように横へずらした、描く列の範囲。
fn visible_columns(screen: &GuitarArticulationScreen, width: u16) -> std::ops::Range<usize> {
    let fit = (usize::from(width).saturating_sub(LABEL_WIDTH) / COLUMN_WIDTH).max(1);
    let first = (screen.cursor() + 1).saturating_sub(fit);
    first..screen.column_count().min(first + fit)
}

fn row(
    label: &str,
    cells: impl Iterator<Item = (&'static str, Style)>,
    cursor: usize,
    first_column: usize,
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{label:<LABEL_WIDTH$}"),
        base_style().fg(MONOKAI_GRAY),
    )];
    for (offset, (mark, style)) in cells.enumerate() {
        let style = if first_column + offset == cursor {
            style.add_modifier(Modifier::REVERSED)
        } else {
            style
        };
        spans.push(Span::styled(mark, style));
        spans.push(Span::styled(" ", base_style()));
    }
    Line::from(spans)
}
