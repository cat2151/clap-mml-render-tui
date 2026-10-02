//! アルペジエーター overlay（`z`）。音型をキー付きで 1 行 1 つ並べ、ON の間は選んでいる音型を反転する。
//! その下に設定と生成した step 数（OFF なら `OFF`）、最後にキーの効き方を出す。

use cmrt_arpeggiator::ArpPattern;
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY},
    ui::centered_rect_with_size,
};

use crate::GuitarArticulationScreen;

/// overlay で音型を直接選ぶキー。並びは overlay の行の並びで、音型は [`crate::ARP_PATTERNS`] と同じ。
pub(crate) const ARP_PATTERN_KEYS: [(char, ArpPattern); 6] = [
    ('u', ArpPattern::Up),
    ('d', ArpPattern::Down),
    ('p', ArpPattern::UpDown),
    ('P', ArpPattern::DownUp),
    ('h', ArpPattern::UpDownHold),
    ('t', ArpPattern::UpTurn),
];

const TITLE: &str = " アルペジエーター(開いている間は隙間なく repeat) ";

/// 設定の行の下に出す、キーの効き方。
const HELP_ROWS: [&str; 8] = [
    " ── keys ──",
    " u d p P h t   音型を選ぶ(OFF なら ON にする)",
    " 1 / 2 / 3     oct を 1 / 2 / 3 にする(音を何オクターブぶん並べるか)",
    " n / N         回数を +1 / -1 (音型を繰り返す回数。1〜8)",
    " b / B         戻り幅を +1 / -1 (UpTurn で下がる音数。1〜3)",
    " 0             OFF にする(値は保つ)",
    " space         頭から鳴らし直す",
    " Enter Esc z   閉じる",
];

/// 音型の下の設定の行。ON なら設定と生成した step 数、OFF なら保持している値。
fn summary(screen: &GuitarArticulationScreen) -> String {
    let arp = screen.arp();
    let values = format!(
        "oct {}  回数 {}  戻り幅 {}",
        arp.octaves, arp.cycles, arp.turn
    );
    if arp.enabled {
        format!(" {values}  step {}", screen.column_count())
    } else {
        format!(" OFF  ({values})")
    }
}

pub(super) fn draw_overlay(f: &mut Frame<'_>, screen: &GuitarArticulationScreen) {
    if !screen.arp_overlay_open() {
        return;
    }
    let arp = screen.arp();
    let mut items: Vec<ListItem> = ARP_PATTERN_KEYS
        .iter()
        .map(|(key, pattern)| ListItem::new(format!(" {key}  {}", pattern.label())))
        .collect();
    items.push(ListItem::new(summary(screen)));
    items.extend(
        HELP_ROWS
            .iter()
            .map(|row| ListItem::new(*row).style(base_style().fg(MONOKAI_GRAY))),
    );
    let selected = arp.enabled.then(|| {
        ARP_PATTERN_KEYS
            .iter()
            .position(|(_, pattern)| *pattern == arp.pattern)
            .unwrap_or(0)
    });
    let width = std::iter::once(TITLE)
        .chain(HELP_ROWS)
        .map(|row| Line::from(row).width() as u16 + 2)
        .max()
        .unwrap_or(0)
        .min(f.area().width);
    let height = (items.len() as u16 + 2).min(f.area().height);
    let area: Rect = centered_rect_with_size(width, height, f.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(TITLE)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    let list = List::new(items)
        .block(block)
        .style(base_style().fg(MONOKAI_FG))
        .highlight_style(cursor_highlight_style(base_style().fg(MONOKAI_FG)));
    let mut state = ListState::default().with_selected(selected);
    f.render_widget(Clear, area);
    f.render_stateful_widget(list, area, &mut state);
}
