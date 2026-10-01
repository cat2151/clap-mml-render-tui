//! 音とルールの matrix。横 = 列（同時刻の note on のまとまり）、縦 = 使われている音高（高い音が上）。
//! 音の段の上に全体の演奏でいま鳴っている列の段、下に行全体のルールの段と、列ごとのルールの段を並べる。

use std::collections::BTreeSet;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK, MONOKAI_YELLOW},
};

use super::rule_rows::RuleRow;
use super::{note_name, RuleGroup, RuleLane, ROW_RULE_ROWS, RULE_LANES, RULE_LIST_KEY, RULE_ROWS};
use crate::auto_pick::picked_columns;
use crate::control::control_rule_affects;
use crate::{Articulation, GuitarArticulationScreen, RowRule, Rule, Take};

/// 1 列の桁（記号 1 つ + 空白）。
const COLUMN_WIDTH: usize = 2;
pub(super) const NOTE_MARK: &str = "■";
pub(super) const NO_NOTE_MARK: &str = "·";
pub(super) const RULE_ON_MARK: &str = "●";
/// 列ルールが ON だが効かない列の記号。
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
/// 全体の演奏でいま鳴っている列の記号と、その段の見出し。
const PLAYHEAD_MARK: &str = "▼";
const PLAYHEAD_LABEL: &str = "play";
const EMPTY_TEXT: &str = "(MML を入れると音が並びます)";

/// 枠の内側に要る行数。演奏位置の段・使われている音高の段・ルールの段。MID モードでは MID の段。
/// 演奏位置の段は鳴っていない間も空けておく。演奏のたびに matrix の高さが変わると下の pane が上下に揺れる。
pub(super) fn height(screen: &GuitarArticulationScreen) -> u16 {
    if let Some(midi) = screen.sample_midi() {
        return super::midi_matrix::height(midi);
    }
    match used_pitches(screen).len() {
        0 => 1,
        pitches => (1 + pitches + ROW_RULE_ROWS.len() + RULE_LANES.len()) as u16,
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
    let label_width = label_width();
    let playhead = screen.playhead_column();
    // 鳴っている間は、鳴っている列が見えるように送る。
    let columns = visible_columns(
        playhead.unwrap_or(screen.cursor()),
        screen.column_count(),
        width,
        label_width,
    );
    let playhead_cells = columns.clone().map(|column| {
        if Some(column) == playhead {
            (PLAYHEAD_MARK, base_style().fg(MONOKAI_YELLOW))
        } else {
            (" ", base_style())
        }
    });
    let mut out = vec![row(
        PLAYHEAD_LABEL,
        MONOKAI_GRAY,
        playhead_cells,
        label_width,
        screen.cursor(),
        columns.start,
    )];
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
            MONOKAI_GRAY,
            cells,
            label_width,
            screen.cursor(),
            columns.start,
        ));
    }
    let auto_picks = picked_columns(screen.notes(), screen.rules());
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
            RuleGroup::Row.color(),
            cells,
            label_width,
            screen.cursor(),
            columns.start,
        ));
    }
    for lane in RULE_LANES {
        let cells = columns
            .clone()
            .map(|column| lane_cell(screen, column, lane));
        let (label, color) = lane_label(screen, lane);
        out.push(row(
            &label,
            color,
            cells,
            label_width,
            screen.cursor(),
            columns.start,
        ));
    }
    out
}

/// 段のルールのうち、列で ON のもの。段の中は排他なので多くて 1 つ。
fn on_in_lane(
    screen: &GuitarArticulationScreen,
    column: usize,
    lane: RuleLane,
) -> Option<&'static RuleRow> {
    lane.rule_rows()
        .find(|rule_row| screen.rules().is_on(column, rule_row.rule))
}

fn lane_cell(
    screen: &GuitarArticulationScreen,
    column: usize,
    lane: RuleLane,
) -> (&'static str, Style) {
    match on_in_lane(screen, column, lane) {
        Some(rule_row) => rule_cell(screen, column, rule_row),
        None => (" ", base_style()),
    }
}

/// 段の見出しと色。複数のルールが入る段は、カーソル列で ON のルールの名前を出す。どれも OFF なら段の名前を灰色で。
fn lane_label(screen: &GuitarArticulationScreen, lane: RuleLane) -> (String, Color) {
    let rule_row = match (lane.idle_name(), on_in_lane(screen, screen.cursor(), lane)) {
        (_, Some(rule_row)) => rule_row,
        (Some(idle_name), None) => return (format!("{RULE_LIST_KEY}:{idle_name}"), MONOKAI_GRAY),
        (None, None) => lane
            .rule_rows()
            .next()
            .expect("1 つしか入らない段にもルールが在る"),
    };
    (
        format!("{}:{}", rule_row.key, rule_row.name),
        rule_row.group.color(),
    )
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
    let mark = first_articulation(screen, column)
        .and_then(stroke_mark)
        .unwrap_or(AUTO_PICK_MARK);
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
    let articulation = articulated.articulation;
    let mark = match (stroke_mark(articulation), articulation, articulated.accent) {
        (Some(stroke), _, _) => stroke,
        (None, Articulation::HammerOn, _) => HAMMER_ON_MARK,
        (None, Articulation::PullOff, _) => PULL_OFF_MARK,
        (None, _, true) => OTHER_ACCENT_MARK,
        (None, _, false) => " ",
    };
    let style = if articulated.accent {
        base_style().fg(MONOKAI_PINK).add_modifier(Modifier::BOLD)
    } else {
        base_style().fg(MONOKAI_GREEN)
    };
    (mark, style)
}

/// ストロークを保つ奏法の D/U。
fn stroke_mark(articulation: Articulation) -> Option<&'static str> {
    match articulation {
        Articulation::SusDown
        | Articulation::MuteDown
        | Articulation::BrushDown
        | Articulation::MuteFretDown => Some(DOWN_STROKE_MARK),
        Articulation::SusUp
        | Articulation::MuteUp
        | Articulation::BrushUp
        | Articulation::MuteFretUp => Some(UP_STROKE_MARK),
        _ => None,
    }
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

/// 列ルールの ON/OFF と効き方の記号。効いていれば overlay の文字をグループの色で、効かない列は灰色の `-`。OFF は空白。
pub(super) fn rule_cell(
    screen: &GuitarArticulationScreen,
    column: usize,
    rule_row: &RuleRow,
) -> (&'static str, Style) {
    let rule = rule_row.rule;
    if !screen.rules().is_on(column, rule) {
        return (" ", base_style());
    }
    if applies(screen, column, rule) {
        (
            letter(rule_row.overlay_key),
            base_style().fg(rule_row.group.color()),
        )
    } else {
        (RULE_IDLE_MARK, base_style().fg(MONOKAI_GRAY))
    }
}

/// overlay の文字（a-zA-Z）を、セルに置ける `&'static str` にする。
fn letter(ch: char) -> &'static str {
    const LETTERS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    LETTERS.find(ch).map_or("?", |i| &LETTERS[i..i + 1])
}

/// 列で ON なのに効いていないルールと、その列の音の奏法と音高。全部効いていれば `None`。
pub(super) fn ineffective_notice(
    screen: &GuitarArticulationScreen,
    column: usize,
) -> Option<String> {
    let rules: Vec<String> = RULE_ROWS
        .iter()
        .filter(|rule_row| {
            screen.rules().is_on(column, rule_row.rule) && !applies(screen, column, rule_row.rule)
        })
        .map(|rule_row| format!("{}:{}", rule_row.overlay_key, rule_row.name))
        .collect();
    if rules.is_empty() {
        return None;
    }
    let notes: Vec<String> = screen
        .notes()
        .iter()
        .zip(screen.articulated())
        .filter(|(note, _)| note.column == column)
        .map(|(note, a)| format!("{} {}", a.articulation.name(), note_name(note.pitch)))
        .collect();
    Some(format!(
        " ! {} は効いていない（この列: {}） ",
        rules.join(", "),
        notes.join(", ")
    ))
}

/// 列の音の奏法に、そのルールが効いているか。ビブラートと効果音は奏法を変えないので、列に音が在れば効く。
/// CC23 / CC32 / CC24 のルールは、効く奏法と音高の音（[`control_rule_affects`]）が列に在れば効く。
fn applies(screen: &GuitarArticulationScreen, column: usize, rule: Rule) -> bool {
    let mut notes_in_column = screen
        .notes()
        .iter()
        .zip(screen.articulated())
        .filter(|(note, _)| note.column == column);
    if matches!(
        rule,
        Rule::LongExtra | Rule::PowerChord | Rule::PositionRelease | Rule::AutoSlideOut
    ) {
        return notes_in_column
            .any(|(note, a)| control_rule_affects(rule, a.articulation, note.pitch));
    }
    let mut in_column = notes_in_column.map(|(_, articulated)| articulated.articulation);
    match rule {
        Rule::Vibrato
        | Rule::EffectHello
        | Rule::EffectResonance
        | Rule::EffectSlideNoise
        | Rule::EffectHardStop => in_column.next().is_some(),
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
        Rule::NaturalHarmonics => in_column.any(|a| a == Articulation::NaturalHarmonics),
        Rule::Brushing => {
            in_column.any(|a| matches!(a, Articulation::BrushDown | Articulation::BrushUp))
        }
        Rule::FretMute => {
            in_column.any(|a| matches!(a, Articulation::MuteFretDown | Articulation::MuteFretUp))
        }
        Rule::SlideOut => in_column.any(|a| a == Articulation::SlideOut),
        Rule::PseudoLegato => in_column.any(|a| a == Articulation::PseudoLegato),
        Rule::Portamento => in_column.any(|a| a == Articulation::Portamento),
        Rule::SlideIn => in_column.any(|a| a == Articulation::SlideIn),
        Rule::TrillHalf => in_column.any(|a| a == Articulation::TrillHalf),
        Rule::TrillWhole => in_column.any(|a| a == Articulation::TrillWhole),
        Rule::TrillMinorThird => in_column.any(|a| a == Articulation::TrillMinorThird),
        Rule::TrillMajorThird => in_column.any(|a| a == Articulation::TrillMajorThird),
        Rule::UnisonBendAuto => in_column.any(|a| a == Articulation::UnisonBendAuto),
        Rule::UnisonBendManual => in_column.any(|a| a == Articulation::UnisonBendManual),
        Rule::ChromaticRun => in_column.any(|a| a == Articulation::ChromaticRun),
        Rule::SlideFxDown => in_column.any(|a| a == Articulation::SlideFxDown),
        Rule::SlideFxUp => in_column.any(|a| a == Articulation::SlideFxUp),
        Rule::SlideFxWow => in_column.any(|a| a == Articulation::SlideFxWow),
        Rule::LongExtra | Rule::PowerChord | Rule::PositionRelease | Rule::AutoSlideOut => false,
    }
}

/// 段の見出しの桁（`F#3` や `a:hammer/pull`）。いちばん長いルールの見出しと、区切りの空白 1 つ。
fn label_width() -> usize {
    let rows = ROW_RULE_ROWS.iter().map(|(_, _, name)| name);
    let rules = RULE_ROWS.iter().map(|rule_row| &rule_row.name);
    rows.chain(rules)
        .map(|name| "k:".len() + name.len() + 1)
        .max()
        .unwrap_or_default()
}

/// `anchor` の列が必ず見えるように横へずらした、描く列の範囲。
pub(super) fn visible_columns(
    anchor: usize,
    column_count: usize,
    width: u16,
    label_width: usize,
) -> std::ops::Range<usize> {
    let fit = (usize::from(width).saturating_sub(label_width) / COLUMN_WIDTH).max(1);
    let first = (anchor + 1).saturating_sub(fit);
    first..column_count.min(first + fit)
}

/// `label_color` は見出しの色。音高の段は灰色、ルールの段はグループの色（[`RuleGroup::color`]）。
pub(super) fn row(
    label: &str,
    label_color: Color,
    cells: impl Iterator<Item = (&'static str, Style)>,
    label_width: usize,
    cursor: usize,
    first_column: usize,
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{label:<label_width$}"),
        base_style().fg(label_color),
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
