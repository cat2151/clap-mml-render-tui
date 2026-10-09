//! Drum kit selector の `?` help overlay。背面の selector と試聴は変えない。

use cmrt_tui_core::{
    help_line::help_line,
    status::base_style,
    theme::MONOKAI_CYAN,
    ui::{centered_text_block_rect, clear_overlay_area},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::{DrumKitSelector, DrumSequencerState};

/// `cmrt_tui_core::buffer_test::help_overlay_bounds` が枠を探す目印なので、
/// 「ヘルプ(Keybinds)」の並びは他画面と揃えたまま変えない。
const TITLE: &str = " Drum kit 選択 ヘルプ(Keybinds)  Esc/?:close ";

pub(super) const ROWS: [&str; 15] = [
    " ── 候補 ──",
    " k/j ↑↓            候補を移る（移ると、その kit を低い note から試聴）",
    " PageUp/PageDown   1 ページ移る",
    " Home/End          先頭 / 末尾",
    " Space             試聴をもう一度",
    " 重い kit          Size がピンクの kit は移っても鳴らさない。Space で読み込んで試聴",
    "                   （読み込みは途中で止められず、終わるまで他の kit も鳴らない）",
    " 行頭の印          … 読み込み中   ♪ 試聴中（状態は一覧の下の行）",
    " ── 絞り込み ──",
    " /                 Regex を編集（空白区切りで AND）",
    " Enter / Esc       （編集中）絞り込みを確定 / 編集前へ戻す",
    " ── 決定 ──",
    " Enter             この kit に決めて閉じる",
    " Esc               取り消して閉じる（編集中の kit は変わらない）",
    " ?                 この help",
];

/// ROWS のキー欄の幅（行頭の空白を含む）。
pub(super) const KEY_COLUMN_WIDTH: usize = 19;

impl DrumSequencerState<'_> {
    pub(super) fn kit_help_open(&self) -> bool {
        self.kit_help_open
    }

    /// help 表示中は全キーを消費し、`?` / Esc でだけ閉じる（q は matrix の help と同じく閉じない）。
    /// 閉じている間は、Regex 編集中を除いて `?` で開く。消費したら true。
    pub(super) fn handle_kit_help_key(&mut self, key: KeyEvent) -> bool {
        let press = key.kind == KeyEventKind::Press;
        if self.kit_help_open {
            if press && matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
                self.kit_help_open = false;
            }
            return true;
        }
        let Some(DrumKitSelector::Select(select)) = &self.selector else {
            return false;
        };
        if select.filter_editing()
            || key.code != KeyCode::Char('?')
            || key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return false;
        }
        if press {
            self.kit_help_open = true;
        }
        true
    }
}

pub(super) fn draw_overlay(frame: &mut Frame<'_>) {
    let lines: Vec<Line<'static>> = ROWS
        .iter()
        .map(|row| help_line(row, KEY_COLUMN_WIDTH))
        .collect();
    let area = centered_text_block_rect(frame.area(), TITLE, &lines);
    clear_overlay_area(frame, area);
    frame.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(TITLE)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}
