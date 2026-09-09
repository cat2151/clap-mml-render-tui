//! `e` / `n` の 1 行入力 overlay の描画。
//!
//! 作りは `grid-sequencer/src/ui/chord_input.rs` と同じ。入力欄 3 行（枠込み）＋
//! 案内 1 行。**理由があるときは 2 行増やして赤字で折り返す**（閉じないので、
//! 直すまで出しっぱなしになる）。折り返すのは、ライブラリのエラー文言が
//! 60 桁に収まらず、1 行で切ると原因の書いてある後半が読めなくなるため。

use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_PINK},
    ui::centered_rect_with_size,
};

use crate::ChordChartScreen;

/// overlay の最大幅。進行は 40 桁もあれば全部見える長さで、これ以上広げても
/// 目が横に泳ぐだけ。
const MAX_WIDTH: u16 = 60;
/// 入力欄（枠込み 3 行）の高さ。
const TEXTAREA_HEIGHT: u16 = 3;
/// 理由の行数。ライブラリのエラー文言は 1 行に収まらないので折り返して 2 行取る。
/// 1 行で切ると「Syntax error in」までしか読めず、どこが悪いのか分からない。
const ERROR_HEIGHT: u16 = 2;
/// 下段の案内。確定と取り消しの両方を必ず出す。
const HINT_HEIGHT: u16 = 1;
const HINT: &str = " Enter:確定  Esc:cancel";

pub(super) fn draw_overlay(f: &mut Frame<'_>, screen: &ChordChartScreen) {
    let Some(input) = screen.line_input() else {
        return;
    };
    let has_error = input.error().is_some();
    let width = f.area().width.saturating_sub(2).min(MAX_WIDTH);
    let height = (TEXTAREA_HEIGHT + if has_error { ERROR_HEIGHT } else { 0 } + HINT_HEIGHT)
        .min(f.area().height);
    let area = centered_rect_with_size(width, height, f.area());
    let textarea_area = Rect::new(area.x, area.y, area.width, TEXTAREA_HEIGHT.min(area.height));
    let below = Rect::new(
        area.x,
        area.y.saturating_add(textarea_area.height),
        area.width,
        area.height.saturating_sub(textarea_area.height),
    );
    // 理由は折り返して出し、案内は必ず最後の 1 行に残す（狭い端末では案内が先に消える）。
    let error_height = below.height.saturating_sub(HINT_HEIGHT).min(ERROR_HEIGHT);
    let error_area = Rect::new(below.x, below.y, below.width, error_height);
    let hint_area = Rect::new(
        below.x,
        below.y.saturating_add(error_height),
        below.width,
        below.height.saturating_sub(error_height),
    );

    f.render_widget(Clear, area);
    let value = input.value();
    let textarea = cmrt_tui_core::text_input::build_query_textarea_widget(
        input.textarea(),
        &value,
        input.target().title(),
        input.target().placeholder(),
        if has_error {
            MONOKAI_PINK
        } else {
            MONOKAI_CYAN
        },
    );
    f.render_widget(&textarea, textarea_area);

    if let Some(error) = input.error() {
        f.render_widget(
            Paragraph::new(Line::styled(
                format!(" ! {error}"),
                base_style().fg(MONOKAI_PINK),
            ))
            .style(base_style())
            .wrap(Wrap { trim: true }),
            error_area,
        );
    }
    f.render_widget(
        Paragraph::new(Line::from(HINT)).style(base_style()),
        hint_area,
    );
    // 点滅する縦線カーソルを入力欄へ置く（app 側の `uses_textarea_cursor` と対）。
    f.set_cursor_position(
        cmrt_tui_core::text_input::single_line_textarea_cursor_position(
            textarea_area,
            input.textarea(),
        ),
    );
}
