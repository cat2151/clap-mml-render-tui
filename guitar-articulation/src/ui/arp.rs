//! アルペジエーター overlay（`z`）。左に param pane、右に素材 pane、最下行にキーと機能名だけを出す。
//! 各行の効き方はヘルプ（`?`）に置く。

use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_PINK},
    ui::centered_rect_with_size,
};

use crate::{ArpRow, GuitarArticulationScreen};

const TITLE: &str = " アルペジエーター ";
const PARAM_KEYS: &str = " j/k:行 h/l:値 H/L:BPM±1 Tab:素材 space:演奏 Esc:閉じる ?:help";
const MATERIAL_KEYS: &str = " Tab/Esc:確定 Enter:改行";
const MATERIALS_TITLE: &str = " 素材 ";
const HELP_TITLE: &str = " アルペジエーター ヘルプ  Esc/?:close ";

/// param pane の幅（右の区切り線を含まない）。
const PARAM_WIDTH: u16 = 28;
/// param pane の行の名前の欄の幅（いちばん長い「アクセント」と値の間に 1 つ空ける）。
const NAME_WIDTH: usize = 11;
/// 素材 pane の最小の幅（左の区切り線を含む）。
const MATERIALS_MIN_WIDTH: u16 = 24;
/// 2 pane の最小の高さ。param pane の全行が入る。
const PANES_MIN_HEIGHT: u16 = 11;

const HELP_ROWS: [&str; 20] = [
    " 開いている間だけ、素材にアルペジエーターを当てて隙間なく repeat する。",
    " 素材と設定は overlay だけのもの。次に開いたとき・起動し直したときに戻る。",
    " 閉じると MML 欄の MML で鳴る。MML 欄・列ごとのルールは書き換えない。",
    " 列ごとのルールは当てない。汚し・奏法・アクセントはメイン画面と共有。",
    " ── param pane ──",
    " 素材    素材 pane のどの行を鳴らすか。h/l で前 / 次(末尾の次は先頭)",
    " 音型    Up / Down / UpDown / DownUp",
    " oct     音を何オクターブぶん並べるか(1〜3)",
    " シフト  声部全体を 1 オクターブずつ下げる / 上げる(-2〜+2)",
    " 下り幅  UpDown で最高音から下りる音数(1〜8・全部)。UpDown のときだけ出る",
    " BPM     40〜240。h/l で ±5、H/L で ±1。chord 素材のときだけ出る",
    " 音価    8 / 8分3連 / 16 / 16分3連 / 32。chord 素材のときだけ出る",
    " 汚し    汚しなし / 汚し&汚しrelease / 汚し / 汚しrelease。メイン画面と共有",
    " 奏法    ピッキング / エコ / オートプリング1 / オートプリング2。メイン画面と共有",
    " アクセント 上 / 下 / 上下。折り返しの頂点 / 谷 / 両方に付ける。メイン画面と共有",
    " MML 素材は MML の t と l で刻み、chord 素材は BPM と音価で刻む。",
    " ── 素材 pane ──",
    " 1 行 1 素材(MML か chord)。Tab / Esc で確定して param pane へ戻り、",
    " カーソル行の素材を鳴らす。空行は捨てる。",
    " 素材リストは保存され、次に起動したときも残る。",
];

/// param pane の行の名前と値。
fn param_row(screen: &GuitarArticulationScreen, row: ArpRow) -> (&'static str, String) {
    let arp = screen.arp();
    match row {
        ArpRow::Material => ("素材", material_value(screen)),
        ArpRow::Pattern => ("音型", arp.pattern.label().to_string()),
        ArpRow::Octaves => ("oct", arp.octaves.to_string()),
        ArpRow::Shift => ("シフト", signed(arp.shift)),
        ArpRow::Down => (
            "下り幅",
            arp.down
                .map_or_else(|| "全部".to_string(), |down| down.to_string()),
        ),
        ArpRow::Bpm => ("BPM", arp.bpm.to_string()),
        ArpRow::Rate => ("音価", arp.rate.name().to_string()),
        ArpRow::Humanize => ("汚し", screen.arp_humanize_label().to_string()),
        ArpRow::Picking => ("奏法", screen.arp_picking_label().to_string()),
        ArpRow::Accent => (
            "アクセント",
            screen.rules().accent_pattern().label().to_string(),
        ),
    }
}

/// 今の素材がリストの何番目か（無ければ `-`）、リストの長さ、今の素材。
fn material_value(screen: &GuitarArticulationScreen) -> String {
    let position = screen
        .arp_material_position()
        .map_or_else(|| "-".to_string(), |index| (index + 1).to_string());
    format!(
        "{position}/{} {}",
        screen.arp_materials().len(),
        screen.arp_material()
    )
}

/// 0 はそのまま、それ以外は符号付き。
fn signed(shift: i8) -> String {
    if shift == 0 {
        "0".to_string()
    } else {
        format!("{shift:+}")
    }
}

fn param_lines(screen: &GuitarArticulationScreen) -> Vec<Line<'static>> {
    let selected = screen
        .arp_materials_input()
        .is_none()
        .then(|| screen.arp_row_index())
        .flatten();
    screen
        .arp_rows()
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let (name, value) = param_row(screen, row);
            let text = format!(" {}{value}", pad(name, NAME_WIDTH));
            if Some(index) == selected {
                Line::styled(text, cursor_highlight_style(base_style().fg(MONOKAI_FG)))
            } else {
                Line::raw(text)
            }
        })
        .collect()
}

/// 表示幅が `width` になるまで右を空白で埋める。
fn pad(text: &str, width: usize) -> String {
    let fill = width.saturating_sub(Line::from(text).width());
    format!("{text}{}", " ".repeat(fill))
}

/// 編集していない間の素材 pane。今の素材の行に `▶`。
fn material_lines(screen: &GuitarArticulationScreen) -> Vec<Line<'static>> {
    if screen.arp_materials().is_empty() {
        return vec![Line::styled(" (Tab で追加)", base_style().fg(MONOKAI_GRAY))];
    }
    let current = screen.arp_material_position();
    screen
        .arp_materials()
        .iter()
        .enumerate()
        .map(|(index, material)| {
            let mark = if Some(index) == current { "▶" } else { " " };
            Line::raw(format!("{mark}{material}"))
        })
        .collect()
}

fn footer(screen: &GuitarArticulationScreen) -> Line<'static> {
    if let Some(error) = &screen.error {
        return Line::styled(format!(" ! {error}"), base_style().fg(MONOKAI_PINK));
    }
    let keys = if screen.arp_materials_input().is_some() {
        MATERIAL_KEYS
    } else {
        PARAM_KEYS
    };
    Line::styled(keys, base_style().fg(MONOKAI_GRAY))
}

fn overlay_size(screen: &GuitarArticulationScreen, area: Rect) -> (u16, u16) {
    let longest = screen
        .arp_materials()
        .iter()
        .map(|material| Line::from(material.as_str()).width() as u16 + 2)
        .max()
        .unwrap_or(0);
    let footer_width = Line::from(PARAM_KEYS).width() as u16;
    let width = (PARAM_WIDTH + longest.max(MATERIALS_MIN_WIDTH))
        .max(footer_width)
        .saturating_add(2)
        .min(area.width);
    let materials = screen.arp_materials().len() as u16 + 1;
    let height = (materials.max(PANES_MIN_HEIGHT) + 3).min(area.height);
    (width, height)
}

fn block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN))
}

pub(super) fn draw_overlay(f: &mut Frame<'_>, screen: &GuitarArticulationScreen) {
    if !screen.arp_overlay_open() {
        return;
    }
    let (width, height) = overlay_size(screen, f.area());
    let area = centered_rect_with_size(width, height, f.area());
    let outer = block(TITLE);
    let inner = outer.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(outer, area);
    let [panes, footer_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);
    let [params, materials] =
        Layout::horizontal([Constraint::Length(PARAM_WIDTH), Constraint::Min(0)]).areas(panes);
    f.render_widget(
        Paragraph::new(param_lines(screen)).style(base_style().fg(MONOKAI_FG)),
        params,
    );
    let editing = screen.arp_materials_input();
    let materials_block = Block::default()
        .borders(Borders::LEFT)
        .title(MATERIALS_TITLE)
        .border_style(base_style().fg(if editing.is_some() {
            MONOKAI_CYAN
        } else {
            MONOKAI_GRAY
        }));
    match editing {
        Some(textarea) => {
            let mut widget = textarea.clone();
            widget.set_block(materials_block);
            widget.set_style(base_style().fg(MONOKAI_FG));
            f.render_widget(&widget, materials);
        }
        None => f.render_widget(
            Paragraph::new(material_lines(screen))
                .style(base_style().fg(MONOKAI_FG))
                .block(materials_block),
            materials,
        ),
    }
    f.render_widget(Paragraph::new(footer(screen)), footer_area);
    if screen.arp_help_open() {
        draw_help(f);
    }
}

fn draw_help(f: &mut Frame<'_>) {
    let width = std::iter::once(HELP_TITLE)
        .chain(HELP_ROWS)
        .map(|row| Line::from(row).width() as u16 + 2)
        .max()
        .unwrap_or(0)
        .min(f.area().width);
    let height = (HELP_ROWS.len() as u16 + 2).min(f.area().height);
    let area = centered_rect_with_size(width, height, f.area());
    let lines: Vec<Line> = HELP_ROWS.iter().copied().map(Line::from).collect();
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .style(base_style().fg(MONOKAI_FG))
            .block(block(HELP_TITLE)),
        area,
    );
}
