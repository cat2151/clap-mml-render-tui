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
use crate::{auto_pick_columns, Articulation, GuitarArticulationScreen, RowRule, Rule, Take};

/// 1 列の桁（記号 1 つ + 空白）。
const COLUMN_WIDTH: usize = 2;
const NOTE_MARK: &str = "■";
const NO_NOTE_MARK: &str = "·";
const RULE_ON_MARK: &str = "●";
/// 列ごとのルールが ON だが、音高や前の列との音程が合わず奏法が変わっていない列。
const RULE_IDLE_MARK: &str = "-";
/// 自動ハンマリングが ON で、ピッキングのまま残すがストロークでない（スライド・チョーキング・PH の）列。
const AUTO_PICK_MARK: &str = "p";
const DOWN_STROKE_MARK: &str = "D";
const UP_STROKE_MARK: &str = "U";
const HAMMER_ON_MARK: &str = "H";
const PULL_OFF_MARK: &str = "P";
/// エコノミーピッキングで、D/U/H/P のどれでもない（スライド・チョーキング・PH の）頂点の音。
const OTHER_ACCENT_MARK: &str = "^";
/// 汚しで、列の発音が書いた時刻より早い / ほぼジャスト / 遅い。
const EARLY_MARK: &str = "<";
const ON_TIME_MARK: &str = "·";
const LATE_MARK: &str = ">";
/// 汚し（リリース）が ON の段の記号。値は下のイベント列の `cc  release type` で見る。
const RELEASE_MARK: &str = "~";
/// 汚しのずれを「ほぼジャスト」とみなす幅（±秒）。
const ON_TIME_SECONDS: f64 = 0.002;
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
    let auto_picks = auto_pick_columns(screen.notes());
    for (rule, key, name) in ROW_RULE_ROWS {
        let on = screen.rules().is_row_on(rule);
        let cells = columns.clone().map(|column| match (on, rule) {
            (false, _) => (" ", base_style()),
            (true, RowRule::AutoHammerPull) => auto_hammer_pull_cell(screen, &auto_picks, column),
            (true, RowRule::EconomyPicking) => economy_picking_cell(screen, column),
            (true, RowRule::Humanize) => humanize_cell(screen, column),
            (true, RowRule::HumanizeRelease) => (RELEASE_MARK, base_style().fg(MONOKAI_GREEN)),
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
            match (
                screen.rules().is_on(column, rule),
                applies(screen, column, rule),
            ) {
                (false, _) => (" ", base_style()),
                (true, true) => (RULE_ON_MARK, base_style().fg(MONOKAI_GREEN)),
                (true, false) => (RULE_IDLE_MARK, base_style().fg(MONOKAI_GRAY)),
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

/// ピッキングする列はストローク（D/U）、レガートの列は ●。和音の列は先頭の音で代表する。
fn auto_hammer_pull_cell(
    screen: &GuitarArticulationScreen,
    auto_picks: &[bool],
    column: usize,
) -> (&'static str, Style) {
    if !auto_picks[column] {
        return (RULE_ON_MARK, base_style().fg(MONOKAI_GREEN));
    }
    let mark = match first_articulation(screen, column) {
        Some(Articulation::SusDown | Articulation::MuteDown) => DOWN_STROKE_MARK,
        Some(Articulation::SusUp | Articulation::MuteUp) => UP_STROKE_MARK,
        _ => AUTO_PICK_MARK,
    };
    (mark, base_style().fg(MONOKAI_GRAY))
}

fn first_articulation(screen: &GuitarArticulationScreen, column: usize) -> Option<Articulation> {
    screen
        .notes()
        .iter()
        .position(|note| note.column == column)
        .and_then(|i| screen.articulated().get(i))
        .map(|articulated| articulated.articulation)
}

/// ストローク（D/U）かレガート（H/P）。頂点のアクセントはピンクの太字。和音の列は先頭の音で代表する。
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
        (Articulation::SusDown | Articulation::MuteDown, _) => DOWN_STROKE_MARK,
        (Articulation::SusUp | Articulation::MuteUp, _) => UP_STROKE_MARK,
        (Articulation::HammerOn, _) => HAMMER_ON_MARK,
        (Articulation::PullOff, _) => PULL_OFF_MARK,
        (_, true) => OTHER_ACCENT_MARK,
        (_, false) => " ",
    };
    let style = if articulated.accent {
        base_style().fg(MONOKAI_PINK).add_modifier(Modifier::BOLD)
    } else {
        base_style().fg(MONOKAI_GREEN)
    };
    (mark, style)
}

/// 列でいちばん早く鳴る音の、汚しによる発音のずれ。
fn humanize_cell(screen: &GuitarArticulationScreen, column: usize) -> (&'static str, Style) {
    let (Some(played), Some(written)) = (
        screen.column_on_seconds(column, Take::Converted),
        screen.column_on_seconds(column, Take::Plain),
    ) else {
        return (" ", base_style());
    };
    let shift = played - written;
    if shift < -ON_TIME_SECONDS {
        (EARLY_MARK, base_style().fg(MONOKAI_GREEN))
    } else if shift > ON_TIME_SECONDS {
        (LATE_MARK, base_style().fg(MONOKAI_GREEN))
    } else {
        (ON_TIME_MARK, base_style().fg(MONOKAI_GRAY))
    }
}

/// 列の音の奏法に、そのルールが効いているか。ビブラートは奏法を変えないので、列に音が在れば効く。
fn applies(screen: &GuitarArticulationScreen, column: usize, rule: Rule) -> bool {
    let mut in_column = screen
        .notes()
        .iter()
        .zip(screen.articulated())
        .filter(|(note, _)| note.column == column)
        .map(|(_, articulated)| articulated.articulation);
    match rule {
        Rule::Vibrato => in_column.next().is_some(),
        Rule::HammerPull => {
            in_column.any(|a| matches!(a, Articulation::HammerOn | Articulation::PullOff))
        }
        Rule::PalmMute => {
            in_column.any(|a| matches!(a, Articulation::MuteDown | Articulation::MuteUp))
        }
        Rule::PinchHarmonic => in_column.any(|a| a == Articulation::PinchHarmonic),
        Rule::PickScratch => in_column.any(|a| a == Articulation::PickScratch),
        Rule::Slide => {
            in_column.any(|a| matches!(a, Articulation::SlideUp | Articulation::SlideDown))
        }
        Rule::Choke => in_column.any(|a| {
            matches!(
                a,
                Articulation::BendHalf | Articulation::BendWhole | Articulation::BendWholeHalf
            )
        }),
    }
}

/// 段の見出しの桁（`F#3` や `a:hammer/pull`）。いちばん長いルールの見出しと、区切りの空白 1 つ。
fn label_width() -> usize {
    let rows = ROW_RULE_ROWS.iter().map(|(_, _, name)| name);
    let rules = RULE_ROWS.iter().map(|(_, _, name)| name);
    rows.chain(rules)
        .map(|name| "k:".len() + name.len() + 1)
        .max()
        .unwrap_or_default()
}

/// カーソル列が必ず見えるように横へずらした、描く列の範囲。
fn visible_columns(screen: &GuitarArticulationScreen, width: u16) -> std::ops::Range<usize> {
    let fit = (usize::from(width).saturating_sub(label_width()) / COLUMN_WIDTH).max(1);
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
        format!("{label:<width$}", width = label_width()),
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
