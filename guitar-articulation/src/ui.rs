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

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_PINK, MONOKAI_YELLOW},
    ui::draw_frame_background,
};

use crate::{Articulation, GuitarArticulationScreen, RowRule, Rule, Take, TimedMidiEvent};

mod help;
mod matrix;

#[cfg(test)]
mod tests;

const KEYBIND_TEXT: &str =
    " h/l:列移動  a:H/P切替  e:eco  s:auto  b:raw  space:Articulated  i:MML入力  q:終了  ?:help";
const INPUT_HINT_TEXT: &str = " MML を編集中  Enter:確定して演奏  Esc:matrix 操作へ  ?:help";
const MML_TITLE: &str = " MML ";
const MML_PLACEHOLDER: &str = "o3 l8 e f+ g";
const MATRIX_TITLE: &str = " 音 / ルール ";
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
    let inner = screen_block().inner(area);
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

fn screen_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(" Guitar Articulation  METAL-GTX Full ")
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_GRAY))
}

fn pane_block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_GRAY))
}

/// キーを受けている pane の枠。
fn focused_pane_block(title: &'static str) -> Block<'static> {
    pane_block(title).border_style(base_style().fg(MONOKAI_CYAN))
}

pub fn draw(screen: &GuitarArticulationScreen, f: &mut Frame<'_>) {
    draw_frame_background(f);
    let layout = layout_for(f.area(), screen);
    f.render_widget(screen_block(), f.area());
    draw_input(f, layout.input, screen);
    let matrix_block = if screen.input_open() {
        pane_block(MATRIX_TITLE)
    } else {
        focused_pane_block(MATRIX_TITLE)
    };
    let matrix_inner = matrix_block.inner(layout.matrix);
    f.render_widget(
        Paragraph::new(matrix::lines(screen, matrix_inner.width)).block(matrix_block),
        layout.matrix,
    );
    draw_events(f, layout.plain, " raw (b) ", screen, Take::Plain);
    draw_events(
        f,
        layout.converted,
        " Articulated (space) ",
        screen,
        Take::Converted,
    );
    f.render_widget(
        Paragraph::new(status_line(screen)).style(base_style()),
        layout.status,
    );
    if screen.help_open() {
        help::draw_overlay(f);
    }
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

fn draw_events(
    f: &mut Frame<'_>,
    area: Rect,
    title: &'static str,
    screen: &GuitarArticulationScreen,
    take: Take,
) {
    let block = pane_block(title);
    let height = block.inner(area).height as usize;
    let events = screen.events(take);
    let target = cursor_event_index(screen, events);
    let first = target.map_or(0, |index| index.saturating_sub(height / 3));
    let lines: Vec<Line> = events
        .iter()
        .skip(first)
        .take(height)
        .enumerate()
        .map(|(offset, event)| {
            let line = Line::from(event_text(event, take == Take::Converted));
            if Some(first + offset) == target {
                line.style(base_style().fg(MONOKAI_YELLOW))
            } else {
                line
            }
        })
        .collect();
    f.render_widget(Paragraph::new(lines).block(block), area);
}

/// カーソル列の最初の note on（同時刻の KS を含む）の位置。
fn cursor_event_index(
    screen: &GuitarArticulationScreen,
    events: &[TimedMidiEvent],
) -> Option<usize> {
    let seconds = screen
        .notes()
        .iter()
        .find(|note| note.column == screen.cursor())?
        .on_seconds;
    events
        .iter()
        .position(|event| is_note_on(event) && (event.seconds - seconds).abs() < 1e-9)
}

fn is_note_on(event: &TimedMidiEvent) -> bool {
    event.message[0] & 0xF0 == 0x90 && event.message[2] != 0
}

/// 1 行 1 イベント: 秒・種類・音名 or KS 名・velocity。
pub(crate) fn event_text(event: &TimedMidiEvent, name_keyswitches: bool) -> String {
    let [status, data1, data2] = event.message;
    let (kind, name) = match status & 0xF0 {
        0x90 if data2 != 0 => ("on", note_label(data1, name_keyswitches)),
        0x80 | 0x90 => ("off", note_label(data1, name_keyswitches)),
        0xB0 => ("cc", format!("CC{data1}")),
        _ => ("?", format!("{status:02X} {data1:02X}")),
    };
    format!("{:>7.3} {kind:<3} {name:<14} {data2:>3}", event.seconds)
}

fn note_label(pitch: u8, name_keyswitches: bool) -> String {
    match Articulation::from_keyswitch(pitch).filter(|_| name_keyswitches) {
        Some(articulation) => format!("KS {}", articulation.name()),
        None => note_name(pitch),
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

/// ルール行の見出しとトグルのキー。
pub(crate) const RULE_ROWS: [(Rule, char, &str); 1] = [(Rule::HammerPull, 'a', "H/P")];

/// 行全体で ON/OFF するルールの段の見出しとトグルのキー。上の段から並べる（H/P の段の上）。
pub(crate) const ROW_RULE_ROWS: [(RowRule, char, &str); 2] = [
    (RowRule::EconomyPicking, 'e', "eco"),
    (RowRule::AutoHammerPull, 's', "auto"),
];

fn status_line(screen: &GuitarArticulationScreen) -> Line<'static> {
    if let Some(error) = &screen.error {
        return Line::from(Span::styled(
            format!(" ! {error}"),
            base_style().fg(MONOKAI_PINK),
        ));
    }
    let text = if screen.input_open() {
        INPUT_HINT_TEXT
    } else {
        KEYBIND_TEXT
    };
    Line::from(Span::styled(text, base_style()))
}
