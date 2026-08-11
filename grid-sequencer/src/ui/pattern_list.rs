//! step セルの wheel が送るフレーズ型の list を、grid の右へ縦に並べる。
//!
//! wheel が「list を送る操作」であることは、送り先の list が見えていて初めて自明になる。
//! 並びが上から下へ見えていれば、ホイールを下へ回すと次へ進むことを覚えなくてよい。
//!
//! カーソルは直近に適用した型（NOTE grid のタイトルに出しているものと同じ）に付ける。
//! まだ一度も回していない section には印が付かないので、未操作だと分かる。

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

use crate::GridSequencerScreen;

/// 枠線ぶん。上下1行ずつ。
const BORDERS: usize = 2;
/// section 見出し1行 + 型の数。
const ARP_LINES: usize = 1 + ArpPattern::ALL.len();
const BASS_LINES: usize = 1 + BassPattern::ALL.len();

/// 中身に必要なぶんだけの高さ（枠線込み）。`available` はこの pane へ割ける行数。
///
/// grid と同じ高さへ引き伸ばすと、広い端末では list の下に長い空白が伸びる。
/// 出す行数は端末の高さで変わらないので、枠を中身へ詰める。
/// bass が落ちる高さでは arp だけのぶんへさらに詰める（[`lines`] と同じ分岐）。
pub(super) fn height_for(available: u16) -> u16 {
    let full = (BORDERS + ARP_LINES + BASS_LINES) as u16;
    if available >= full {
        return full;
    }
    ((BORDERS + ARP_LINES) as u16).min(available)
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

/// 両方の section を出す。入りきらないときは bass を落として arp を優先する。
///
/// arp のほうが型が多く、4 voice 行のほうが触る機会も多い。どちらも入らない高さでは
/// 上から順に切れるが、そこまで低い端末では grid 自体が成立していない。
fn lines(screen: &GridSequencerScreen, height: usize) -> Vec<Line<'static>> {
    let available = height.saturating_sub(BORDERS);
    let mut lines = section(
        "arp",
        ArpPattern::ALL.iter().map(|pattern| pattern.label()),
        screen.last_arp().map(ArpPattern::label),
    );
    if available >= ARP_LINES + BASS_LINES {
        lines.extend(section(
            "bass",
            BassPattern::ALL.iter().map(|pattern| pattern.label()),
            screen.last_bass().map(BassPattern::label),
        ));
    }
    lines
}

fn section(
    name: &str,
    labels: impl Iterator<Item = &'static str>,
    current: Option<&'static str>,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::styled(
        name.to_string(),
        base_style().fg(MONOKAI_GRAY),
    )];
    lines.extend(labels.map(|label| {
        let selected = current == Some(label);
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
