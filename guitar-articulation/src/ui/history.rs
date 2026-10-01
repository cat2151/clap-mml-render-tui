//! history overlay（`Shift+H`）。履歴を 1 行 1 件で新しい順に並べ、選んでいる行を反転する。

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_FG},
    ui::centered_rect_with_size,
};

use crate::{GuitarArticulationHistoryEntry, GuitarArticulationScreen, RowRule, Rule};

const TITLE: &str =
    " Guitar Articulation History  j/k,↑/↓:選んで演奏  Enter:確定  Esc/Shift+H:取り消し ";

/// 行全体のルールの、1 行に載せる短い名前。
const ROW_RULE_LABELS: [(RowRule, &str); 4] = [
    (RowRule::EconomyPicking, "eco"),
    (RowRule::AutoHammerPull, "auto"),
    (RowRule::Humanize, "汚し"),
    (RowRule::HumanizeRelease, "汚しrel"),
];

/// 列ごとのルールの、1 行に載せる短い名前。
const RULE_LABELS: [(Rule, &str); 31] = [
    (Rule::HammerPull, "H/P"),
    (Rule::PalmMute, "mute"),
    (Rule::PinchHarmonic, "PH"),
    (Rule::Slide, "slide"),
    (Rule::Choke, "bend"),
    (Rule::Vibrato, "vib"),
    (Rule::PickScratch, "scratch"),
    (Rule::NaturalHarmonics, "NH"),
    (Rule::Brushing, "brush"),
    (Rule::FretMute, "fretmute"),
    (Rule::SlideOut, "slideout"),
    (Rule::PseudoLegato, "legato"),
    (Rule::Portamento, "porta"),
    (Rule::SlideIn, "slidein"),
    (Rule::TrillHalf, "trill1"),
    (Rule::TrillWhole, "trill2"),
    (Rule::TrillMinorThird, "trill3"),
    (Rule::TrillMajorThird, "trill4"),
    (Rule::UnisonBendAuto, "unison"),
    (Rule::UnisonBendManual, "unisonpb"),
    (Rule::ChromaticRun, "chrun"),
    (Rule::SlideFxDown, "fxdown"),
    (Rule::SlideFxUp, "fxup"),
    (Rule::SlideFxWow, "fxwow"),
    (Rule::EffectHello, "hello"),
    (Rule::EffectResonance, "reso"),
    (Rule::EffectSlideNoise, "slidenoise"),
    (Rule::EffectHardStop, "hardstop"),
    (Rule::LongExtra, "long"),
    (Rule::PowerChord, "p5"),
    (Rule::PositionRelease, "posrel"),
];

pub(super) fn draw_overlay(f: &mut Frame<'_>, screen: &GuitarArticulationScreen) {
    let Some(selected) = screen.history_overlay_selected() else {
        return;
    };
    let entries = &screen.history().entries;
    let area = overlay_rect(f.area(), entries.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(TITLE)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| ListItem::new(row_text(i, entry)))
        .collect();
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(block)
        .style(base_style().fg(MONOKAI_FG))
        .highlight_style(cursor_highlight_style(base_style().fg(MONOKAI_FG)));
    f.render_widget(Clear, area);
    f.render_stateful_widget(list, area, &mut state);
}

/// `#01  <MML>  eco auto H/P:3 cc22=0  fx:2`。MML が同じで設定だけ違う履歴を見分けられるよう、
/// ON の行ルール・列ルールごとの列数・既定と違うパラメータ・chain の段数を後ろに付ける。
pub(super) fn row_text(index: usize, entry: &GuitarArticulationHistoryEntry) -> String {
    let mut settings: Vec<String> = ROW_RULE_LABELS
        .iter()
        .filter(|(rule, _)| entry.rules.is_row_on(*rule))
        .map(|(_, label)| label.to_string())
        .collect();
    settings.extend(RULE_LABELS.iter().filter_map(|(rule, label)| {
        let count = entry.rules.column_count_of(*rule);
        (count > 0).then(|| format!("{label}:{count}"))
    }));
    settings.extend(
        entry
            .rules
            .changed_params()
            .map(|(cc, value)| format!("cc{cc}={value}")),
    );
    let mut text = format!("#{:02}  {}", index + 1, entry.mml);
    if !settings.is_empty() {
        text.push_str("  ");
        text.push_str(&settings.join(" "));
    }
    if !entry.effect_chain.is_empty() {
        text.push_str(&format!("  fx:{}", entry.effect_chain.len()));
    }
    text
}

fn overlay_rect(area: Rect, row_count: usize) -> Rect {
    let width = area.width.saturating_sub(4).min(120);
    let height = u16::try_from(row_count.max(1) + 2)
        .unwrap_or(u16::MAX)
        .min(area.height.saturating_sub(2))
        .max(3);
    centered_rect_with_size(width, height, area)
}
