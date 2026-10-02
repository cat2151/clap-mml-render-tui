//! Guitar Articulation 画面の描画。
//!
//! 縦に「MML 入力欄 / 音とルールの matrix / raw と Articulated のイベント列（左右） / 下段 1 行」。
//! キーを受ける側（MML 欄か matrix）の枠だけを水色にする。全部が灰色だと、端末が
//! focus を失って灰色になったのと見分けがつかない。help は最後に overlay として重ねる。

use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use cmrt_arpeggiator::ArpPattern;
use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_PINK},
    ui::draw_frame_background,
};

use crate::{ArpSettings, AutoPick, GuitarArticulationScreen, RowRule, Take};

mod arp;
mod event_list;
mod help;
mod history;
mod matrix;
mod midi_matrix;
mod param_list;
mod rule_list;
mod rule_rows;
mod sample_midi;

pub(crate) use arp::ARP_PATTERN_KEYS;
pub(crate) use event_list::{name_width, EventRow};
pub(crate) use rule_rows::{
    RuleGroup, RuleLane, RuleRow, ROW_RULE_ROWS, RULE_LANES, RULE_LIST_KEY, RULE_ROWS,
};

#[cfg(test)]
mod tests;

/// `b` / `space` / `i` は raw / Articulated / MML の pane の見出しに出ているので、ここには載せない（幅が足りない）。
/// 幅 100 の端末で `?:help` まで収まる長さに保つ（枠の内側 98 桁）。
const KEYBIND_TEXT: &str =
    "h/l:移動 a:H/P mp/cvg/t:奏法 u:値 e:eco s:auto d:汚し r:rel n:1音 x/w:fx H:履歴 o:MID q:終 ?:help";
const MID_KEYBIND_TEXT: &str =
    " o:MID選択 space:演奏 n:1音 h/l:1音ずつ Esc:MIDを閉じる q:終了 ?:help";
const INPUT_HINT_TEXT: &str = " MML を編集中  Enter:確定して演奏  Esc:matrix 操作へ  ?:help";
const MML_TITLE: &str = " MML (i) ";
const MML_PLACEHOLDER: &str = "o3 l8 e f+ g";
const MATRIX_TITLE: &str = " 音 / ルール ";
const PLAIN_TITLE: &str = " raw (b) ";
/// 入力欄（枠込み）の高さ。
const INPUT_HEIGHT: u16 = 3;
/// イベント列の pane に最低限残す高さ（枠込み）。matrix が高くてもこれだけは残す。
const EVENTS_MIN_HEIGHT: u16 = 5;

/// 描画とテストが同じ矩形を見るための、唯一の layout の作り方。
pub(crate) struct GuitarArticulationLayout {
    pub input: Rect,
    pub matrix: Rect,
    pub plain: Rect,
    pub converted: Rect,
    pub status: Rect,
}

pub(crate) fn layout_for(
    area: Rect,
    screen: &GuitarArticulationScreen,
) -> GuitarArticulationLayout {
    let inner = screen_block(String::new()).inner(area);
    let matrix_height = matrix::height(screen) + 2;
    let rows = Layout::vertical([
        Constraint::Length(INPUT_HEIGHT),
        Constraint::Max(matrix_height),
        Constraint::Min(EVENTS_MIN_HEIGHT),
        Constraint::Length(1),
    ])
    .split(inner);
    let panes =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[2]);
    GuitarArticulationLayout {
        input: rows[0],
        matrix: rows[1],
        plain: panes[0],
        converted: panes[1],
        status: rows[3],
    }
}

fn screen_block(title: String) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_GRAY))
}

fn pane_block(title: impl Into<Line<'static>>) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_GRAY))
}

/// キーを受けている pane の枠。
fn focused_pane_block(title: impl Into<Line<'static>>) -> Block<'static> {
    pane_block(title).border_style(base_style().fg(MONOKAI_CYAN))
}

pub fn draw(screen: &GuitarArticulationScreen, f: &mut Frame<'_>) {
    draw_frame_background(f);
    let layout = layout_for(f.area(), screen);
    f.render_widget(screen_block(screen_title(screen)), f.area());
    if screen.sample_midi().is_some() {
        sample_midi::draw_panes(f, &layout, screen);
    } else {
        draw_mml_panes(f, &layout, screen);
    }
    f.render_widget(
        Paragraph::new(status_line(screen)).style(base_style()),
        layout.status,
    );
    if let Some((editor, adding)) = screen.effect_overlay() {
        cmrt_effect_chain_select::draw(
            f,
            f.area(),
            editor,
            screen.effect_catalog(),
            cmrt_effect_chain_select::EffectChainView {
                adding,
                header: Line::from(format!("instrument: {}", screen.instrument().label())),
                error: None,
            },
        );
    }
    history::draw_overlay(f, screen);
    sample_midi::draw_list_overlay(f, screen);
    rule_list::draw_overlay(f, screen);
    param_list::draw_overlay(f, screen);
    arp::draw_overlay(f, screen);
    if screen.help_open() {
        help::draw_overlay(f);
    }
}

/// MML 欄・matrix・raw と Articulated のイベント列。
fn draw_mml_panes(
    f: &mut Frame<'_>,
    layout: &GuitarArticulationLayout,
    screen: &GuitarArticulationScreen,
) {
    draw_input(f, layout.input, screen);
    let matrix_block = if screen.input_open() {
        pane_block(matrix_title(screen))
    } else {
        focused_pane_block(matrix_title(screen))
    };
    let matrix_inner = matrix_block.inner(layout.matrix);
    f.render_widget(
        Paragraph::new(matrix::lines(screen, matrix_inner.width)).block(matrix_block),
        layout.matrix,
    );
    event_list::draw(f, layout.plain, PLAIN_TITLE, screen, Take::Plain);
    event_list::draw(
        f,
        layout.converted,
        " Articulated (space) ",
        screen,
        Take::Converted,
    );
}

/// カーソル列に効いていないルールがあれば、見出しの後ろにピンクで足す。
fn matrix_title(screen: &GuitarArticulationScreen) -> Line<'static> {
    let mut spans = vec![Span::raw(MATRIX_TITLE)];
    if let Some(notice) = matrix::ineffective_notice(screen, screen.cursor()) {
        spans.push(Span::styled(notice, base_style().fg(MONOKAI_PINK)));
    }
    Line::from(spans)
}

/// 音色の後ろに、確定済みの chain を信号の順に並べる。dry の間も chain は出したまま ` [dry]` を足す。
fn screen_title(screen: &GuitarArticulationScreen) -> String {
    let mode = if screen.sample_midi().is_some() {
        " [MID]"
    } else {
        ""
    };
    let mut title = format!(
        " Guitar Articulation{mode}  {}",
        screen.instrument().label()
    );
    // 空の chain で catalog を引くと、未走査のときに描画が走査を待ってしまう。
    if !screen.effect_chain().is_empty() {
        let catalog = screen.effect_catalog();
        for stage in screen.effect_chain() {
            title.push_str(" → ");
            title.push_str(&cmrt_effect_chain_select::stage_label(stage, catalog));
        }
    }
    if screen.effect_dry() {
        title.push_str(" [dry]");
    }
    if screen.note_preview() {
        title.push_str(" [1音]");
    }
    if screen.repeat() {
        title.push_str(" [repeat]");
    }
    if let Some(arp) = arp_flag(screen.arp()) {
        title.push_str(&arp);
    }
    let rules = screen.rules();
    let auto2 = rules.is_row_on(RowRule::AutoHammerPull) && rules.auto_pick() == AutoPick::Accent;
    if auto2 {
        title.push_str(" [auto2]");
    }
    // アクセントは強弱を付ける行ルール（e / d）か、自動ハンマリングの on2 の間だけ効く。
    if auto2 || rules.is_row_on(RowRule::EconomyPicking) || rules.is_row_on(RowRule::Humanize) {
        title.push_str(&format!(" [accent:{}]", rules.accent_pattern().label()));
    }
    title.push_str(&format!(" [起動:{}]", screen.startup_instrument().label()));
    title.push(' ');
    title
}

/// アルペジエーターが ON の間だけ ` [arp:音型 xオクターブ N回数 b戻り幅]`。戻り幅は UpTurn のときだけ。
fn arp_flag(arp: &ArpSettings) -> Option<String> {
    if !arp.enabled {
        return None;
    }
    let turn = if arp.pattern == ArpPattern::UpTurn {
        format!(" b{}", arp.turn)
    } else {
        String::new()
    };
    Some(format!(
        " [arp:{} x{} N{}{turn}]",
        arp.pattern.label(),
        arp.octaves,
        arp.cycles
    ))
}

fn draw_input(f: &mut Frame<'_>, area: Rect, screen: &GuitarArticulationScreen) {
    match screen.input() {
        Some(textarea) => {
            let value = cmrt_tui_core::text_input::textarea_value(textarea);
            let border = if screen.error.is_some() {
                MONOKAI_PINK
            } else {
                MONOKAI_CYAN
            };
            let widget = cmrt_tui_core::text_input::build_query_textarea_widget(
                textarea,
                &value,
                MML_TITLE,
                MML_PLACEHOLDER,
                border,
            );
            f.render_widget(&widget, area);
            // 点滅する縦線カーソルを入力欄へ置く（app 側の `uses_textarea_cursor` と対）。
            f.set_cursor_position(
                cmrt_tui_core::text_input::single_line_textarea_cursor_position(area, textarea),
            );
        }
        None => {
            let text = if screen.mml().is_empty() {
                Span::styled("(i キーで MML 入力開始)", base_style().fg(MONOKAI_GRAY))
            } else {
                Span::raw(screen.mml().to_string())
            };
            f.render_widget(
                Paragraph::new(Line::from(text)).block(pane_block(MML_TITLE)),
                area,
            );
        }
    }
}

/// `60` = `C4`。
pub(crate) fn note_name(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = i32::from(pitch) / 12 - 1;
    format!("{}{octave}", NAMES[usize::from(pitch % 12)])
}

/// パラメータ overlay を開くキー。
pub(crate) const PARAM_LIST_KEY: char = 'u';

fn status_line(screen: &GuitarArticulationScreen) -> Line<'static> {
    if let Some(error) = &screen.error {
        return Line::from(Span::styled(
            format!(" ! {error}"),
            base_style().fg(MONOKAI_PINK),
        ));
    }
    let text = if screen.input_open() {
        INPUT_HINT_TEXT
    } else if screen.sample_midi().is_some() {
        MID_KEYBIND_TEXT
    } else {
        KEYBIND_TEXT
    };
    Line::from(Span::styled(text, base_style()))
}
