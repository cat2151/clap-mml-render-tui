//! step セルの wheel が送るフレーズ型のlistを、gridの右へ並べる。
//!
//! wheel が「list を送る操作」であることは、送り先の list が見えていて初めて自明になる。
//! 並びが上から下へ見えていれば、ホイールを下へ回すと次へ進むことを覚えなくてよい。
//!
//! カーソルは直近に適用した型（NOTE grid のタイトルに出しているものと同じ）に付ける。
//! まだ一度も回していない section には印が付かないので、未操作だと分かる。
//!
//! arp+bassとdrumは別の縦列にして、限られた高さでも全roleを同時に見せる。

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN},
};

use cmrt_arpeggiator::{ArpPattern, BassPattern};
use cmrt_rhythm::{DrumPattern, DrumRole};

use crate::GridSequencerScreen;

/// 枠線ぶん。上下1行ずつ。
const BORDERS: usize = 2;

/// 1 section ぶんの中身。見出し1行＋型の並び。
struct Section {
    name: String,
    labels: Vec<&'static str>,
    current: Option<&'static str>,
}

impl Section {
    fn height(&self) -> usize {
        1 + self.labels.len()
    }
}

/// 左にarp+bass、右にdrum全roleの列を組み立てる。片方が無ければ1列で使う。
fn section_columns(screen: &GridSequencerScreen) -> Vec<Vec<Section>> {
    let mut phrase_sections = Vec::new();
    if screen.state.chord().is_some() {
        phrase_sections.push(Section {
            name: "arp".to_string(),
            labels: ArpPattern::ALL
                .iter()
                .map(|pattern| pattern.label())
                .collect(),
            current: screen.last_arp().map(ArpPattern::label),
        });
        phrase_sections.push(Section {
            name: "bass".to_string(),
            labels: BassPattern::ALL
                .iter()
                .map(|pattern| pattern.label())
                .collect(),
            current: screen.last_bass().map(BassPattern::label),
        });
    }

    let mut drum_sections = Vec::new();
    // drum は1候補しかないroleも省略しない。画面の下から上（kick→snare→hat→perc）の
    // 順に置き、各行のwheelの送り先を常に確認できるようにする。
    for role in drum_section_roles(screen) {
        drum_sections.push(Section {
            name: format!("drum {}", role.label().to_lowercase()),
            labels: DrumPattern::all_for(role)
                .iter()
                .map(|pattern| pattern.label())
                .collect(),
            current: screen.last_drum_for(role).map(DrumPattern::label),
        });
    }

    [phrase_sections, drum_sections]
        .into_iter()
        .filter(|sections| !sections.is_empty())
        .collect()
}

/// 画面に存在するdrum roleを、下の行から上の行の順に返す。
fn drum_section_roles(screen: &GridSequencerScreen) -> Vec<DrumRole> {
    (0..screen.state.instance_count())
        .rev()
        .filter_map(|instance| screen.state.drum_role(instance))
        .fold(Vec::new(), |mut roles, role| {
            if !roles.contains(&role) {
                roles.push(role);
            }
            roles
        })
}

/// 各列の行数。paneの高さは最も長い列へ合わせる。
pub(crate) fn section_heights(screen: &GridSequencerScreen) -> Vec<usize> {
    section_columns(screen)
        .iter()
        .map(|sections| sections.iter().map(Section::height).sum())
        .collect::<Vec<_>>()
}

/// 中身に必要なぶんだけの高さ（枠線込み）。0 なら pane を出さない。
///
/// gridと同じ高さへ引き伸ばさず、最長列の中身へ詰める。
pub(super) fn height_for(section_heights: &[usize], available: u16) -> u16 {
    let maximum = usize::from(available).saturating_sub(BORDERS);
    let content = section_heights
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .min(maximum);
    if content == 0 {
        return 0;
    }
    u16::try_from(BORDERS + content).unwrap_or(u16::MAX)
}

pub(super) fn draw(screen: &GridSequencerScreen, f: &mut Frame<'_>, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Phrase ")
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut columns = section_columns(screen);
    // 2列ぶんの幅が無ければ従来幅へ畳む。drum全roleを先に置き、そのあとへ
    // arp+bassを続けることで、今回の主目的であるdrum listを狭い画面でも欠かさない。
    if inner.width < crate::ui::layout::PATTERN_LIST_WIDTH - BORDERS as u16 && columns.len() == 2 {
        let phrases = columns.remove(0);
        let mut drums = columns.remove(0);
        drums.extend(phrases);
        columns.push(drums);
    }
    let constraints = vec![Constraint::Ratio(1, columns.len() as u32); columns.len()];
    let areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);
    for (sections, area) in columns.iter().zip(areas.iter()) {
        f.render_widget(
            Paragraph::new(lines_for_column(sections, usize::from(area.height)))
                .style(base_style()),
            *area,
        );
    }
}

#[cfg(test)]
fn lines(screen: &GridSequencerScreen, height: usize) -> Vec<Line<'static>> {
    let available = height.saturating_sub(BORDERS);
    section_columns(screen)
        .iter()
        .flat_map(|sections| lines_for_column(sections, available))
        .collect()
}

fn lines_for_column(sections: &[Section], available: usize) -> Vec<Line<'static>> {
    sections.iter().flat_map(render).take(available).collect()
}

fn render(section: &Section) -> Vec<Line<'static>> {
    let mut lines = vec![Line::styled(
        section.name.clone(),
        base_style().fg(MONOKAI_GRAY),
    )];
    lines.extend(section.labels.iter().map(|label| {
        let selected = section.current == Some(*label);
        let marker = if selected { "> " } else { "  " };
        Line::styled(format!("{marker}{label}"), entry_style(selected))
    }));
    lines
}

fn entry_style(selected: bool) -> Style {
    if selected {
        base_style().fg(MONOKAI_GREEN)
    } else {
        base_style()
    }
}

#[cfg(test)]
mod tests;
