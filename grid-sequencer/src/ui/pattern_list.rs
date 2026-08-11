//! step セルの wheel が送るフレーズ型の list を、grid の右へ縦に並べる。
//!
//! wheel が「list を送る操作」であることは、送り先の list が見えていて初めて自明になる。
//! 並びが上から下へ見えていれば、ホイールを下へ回すと次へ進むことを覚えなくてよい。
//!
//! カーソルは直近に適用した型（NOTE grid のタイトルに出しているものと同じ）に付ける。
//! まだ一度も回していない section には印が付かないので、未操作だと分かる。
//!
//! 出す section は行の構成で変わる（arp / bass は chord mode 中だけ、drum は drum 行が
//! あるとき）。高さが足りないときは後ろの section から落とす。

use ratatui::{
    layout::Rect,
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

/// 出す section を優先順（＝上から並べる順）に組み立てる。
///
/// 高さが足りないときに落とすのは後ろから。arp のほうが型が多く、4 voice 行のほうが
/// 触る機会も多いので先頭へ置く。
fn sections(screen: &GridSequencerScreen) -> Vec<Section> {
    let mut sections = Vec::new();
    if screen.state.chord().is_some() {
        sections.push(Section {
            name: "arp".to_string(),
            labels: ArpPattern::ALL
                .iter()
                .map(|pattern| pattern.label())
                .collect(),
            current: screen.last_arp().map(ArpPattern::label),
        });
        sections.push(Section {
            name: "bass".to_string(),
            labels: BassPattern::ALL
                .iter()
                .map(|pattern| pattern.label())
                .collect(),
            current: screen.last_bass().map(BassPattern::label),
        });
    }
    if let Some(role) = drum_section_role(screen) {
        sections.push(Section {
            name: format!("drum {}", role.label().to_lowercase()),
            labels: DrumPattern::all_for(role)
                .iter()
                .map(|pattern| pattern.label())
                .collect(),
            current: screen
                .last_drum()
                .filter(|pattern| pattern.role() == role)
                .map(DrumPattern::label),
        });
    }
    sections
}

/// drum の list を出す役割。型ごとに list が違うので、1つに絞って出す。
///
/// wheel を回したあとはその行の役割、まだ回していなければ画面のいちばん下の
/// drum 行（＝ kick）の list を出す。
fn drum_section_role(screen: &GridSequencerScreen) -> Option<DrumRole> {
    if let Some(pattern) = screen.last_drum() {
        return Some(pattern.role());
    }
    (0..screen.state.instance_count())
        .rev()
        .find_map(|instance| screen.state.drum_role(instance))
}

/// 出す section それぞれの行数を、優先順に並べたもの。
///
/// 高さの計算（[`height_for`]）と描画（[`lines`]）が同じ判断をするための唯一の入口。
pub(crate) fn section_heights(screen: &GridSequencerScreen) -> Vec<usize> {
    sections(screen)
        .iter()
        .map(Section::height)
        .collect::<Vec<_>>()
}

/// 中身に必要なぶんだけの高さ（枠線込み）。0 なら pane を出さない。
///
/// grid と同じ高さへ引き伸ばすと、広い端末では list の下に長い空白が伸びる。
/// 出す行数は端末の高さで変わらないので、枠を中身へ詰める。
/// 入りきらないぶんは後ろの section から落とす（[`lines`] と同じ分岐）。
pub(super) fn height_for(section_heights: &[usize], available: u16) -> u16 {
    let content = fitted(
        section_heights,
        usize::from(available).saturating_sub(BORDERS),
    );
    if content == 0 {
        return 0;
    }
    u16::try_from(BORDERS + content).unwrap_or(u16::MAX)
}

/// 前から順に入るところまで採用したときの合計行数。
///
/// 1つも入らない高さでも、先頭の section だけは切り詰めて出す。まるごと消すと
/// 「送り先の list がある」こと自体が見えなくなる。
fn fitted(section_heights: &[usize], available: usize) -> usize {
    let mut used = 0;
    for height in section_heights {
        if used + height > available {
            break;
        }
        used += height;
    }
    if used == 0 {
        return section_heights.first().copied().unwrap_or(0).min(available);
    }
    used
}

pub(super) fn draw(screen: &GridSequencerScreen, f: &mut Frame<'_>, area: Rect) {
    let lines = lines(screen, usize::from(area.height));
    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Phrase ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

/// 入りきる section だけを上から並べる。[`fitted`] と同じ判断をすること。
fn lines(screen: &GridSequencerScreen, height: usize) -> Vec<Line<'static>> {
    let available = height.saturating_sub(BORDERS);
    let sections = sections(screen);
    let mut lines = Vec::new();
    let mut used = 0;
    for section in &sections {
        if used + section.height() > available {
            break;
        }
        used += section.height();
        lines.extend(render(section));
    }
    if lines.is_empty() {
        if let Some(section) = sections.first() {
            lines.extend(render(section));
            lines.truncate(available);
        }
    }
    lines
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
