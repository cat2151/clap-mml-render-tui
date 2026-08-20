//! play server が起動できないことを画面に出す知らせ。
//!
//! これが無かったころ、install 済みの古い server exe を掴んで即死したときに
//! ユーザーが得られる情報は「音が鳴らない」だけだった（理由は子の stderr にあり、
//! `log/log.txt` を読むまで分からなかった）。理由は
//! [`cmrt_realtime_play::ServerStartupFailure`] が持っているので、ここは
//! 「いつ出すか」「どう描くか」だけを持つ。
//!
//! 出したままにはしない。どのキーでも閉じられ、閉じたあとは同じ理由では出さない。
//! サーバーが起動できたら supervisor 側で理由が消えるので、この知らせも自然に消える。

use cmrt_realtime_play::ServerStartupFailure;
use ratatui::{
    layout::Alignment,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_GRAY, MONOKAI_PINK, MONOKAI_YELLOW},
    ui::centered_text_block_rect,
};

use super::TuiApp;

const TITLE: &str = " play server ";
const DISMISS_HINT: &str = "何かキーを押すと閉じます";

/// 枠と左右の余白ぶん。これを引いた幅で折り返す。
const FRAME_MARGIN: u16 = 6;

/// 折り返し後の最大幅。端末が横に広くても、読める幅で止める。
const MAX_NOTICE_WIDTH: usize = 96;

impl TuiApp<'_> {
    /// いま出すべき知らせ。閉じられた理由と同じなら出さない。
    pub(in crate::tui) fn play_server_notice(&self) -> Option<ServerStartupFailure> {
        let failure = self.play_server.last_startup_failure()?;
        (self.dismissed_play_server_failure.as_ref() != Some(&failure)).then_some(failure)
    }

    /// 知らせを閉じる。閉じるものが無ければ `false`（キーは画面側へ通す）。
    pub(in crate::tui) fn dismiss_play_server_notice(&mut self) -> bool {
        let Some(failure) = self.play_server_notice() else {
            return false;
        };
        self.dismissed_play_server_failure = Some(failure);
        true
    }
}

pub(super) fn draw(f: &mut Frame<'_>, failure: &ServerStartupFailure) {
    let width = notice_width(f.area().width);
    let lines = notice_lines(failure, width);
    let area = centered_text_block_rect(f.area(), TITLE, &lines);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(TITLE)
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_PINK)),
            ),
        area,
    );
}

fn notice_width(area_width: u16) -> usize {
    usize::from(area_width.saturating_sub(FRAME_MARGIN)).min(MAX_NOTICE_WIDTH)
}

/// 1 行目だけ強調する。狭い端末で下が切れても、何が起きたかは残る。
fn notice_lines(failure: &ServerStartupFailure, width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    for (index, text) in failure.lines().into_iter().enumerate() {
        let style = if index == 0 {
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD)
        } else {
            base_style()
        };
        lines.extend(
            wrap_to_width(&text, width)
                .into_iter()
                .map(|wrapped| Line::from(Span::styled(wrapped, style))),
        );
    }
    lines.push(Line::from(Span::styled(
        DISMISS_HINT,
        base_style().fg(MONOKAI_GRAY),
    )));
    lines
}

/// 表示幅で折り返す。
///
/// 幅は文字数ではなく端末上の桁数で数える（日本語 1 文字が 2 桁ぶんを占めるため）。
/// 桁数の計算は ratatui の [`Span::width`] に任せる。この 1 か所のためだけに
/// unicode 幅の crate を足さない。
fn wrap_to_width(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_owned()];
    }
    let mut wrapped = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    for character in text.chars() {
        let character_width = Span::raw(character.to_string()).width();
        if current_width + character_width > width && !current.is_empty() {
            wrapped.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(character);
        current_width += character_width;
    }
    if wrapped.is_empty() || !current.is_empty() {
        wrapped.push(current);
    }
    wrapped
}

#[cfg(test)]
mod tests;
