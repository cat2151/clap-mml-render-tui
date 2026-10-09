//! Regex 欄と、この selector モードで使えるキーの案内。

use super::*;
use cmrt_tui_core::text_input::{
    build_query_textarea_widget, single_line_textarea_cursor_position, textarea_value,
};

const QUERY_PLACEHOLDER: &str = r"例: warm pad|strings";

pub(super) fn draw_query(
    select: &PatchSelect<'_>,
    frame: &mut Frame<'_>,
    area: Rect,
    options: &PatchSelectDrawOptions<'_>,
) {
    let textarea = select.query_textarea();
    let value = textarea_value(textarea);
    let (title, placeholder, border_color) = if select.filter_editing() {
        (
            " Regex (空白=AND)  Enter:絞り込み確定  Esc:前回へ戻す ".to_string(),
            QUERY_PLACEHOLDER,
            MONOKAI_YELLOW,
        )
    } else {
        (
            query_title(select, options.show_play_settings_hint),
            "/ で絞り込み",
            MONOKAI_FG,
        )
    };
    frame.render_widget(
        &build_query_textarea_widget(textarea, &value, &title, placeholder, border_color),
        area,
    );
    if select.filter_editing() {
        // 編集中だけ端末カーソルを Regex 欄へ移す。通常時は pane 操作中だと分かるよう、
        // 呼び出し元の MML カーソルを上書きしない。
        frame.set_cursor_position(single_line_textarea_cursor_position(area, textarea));
    }
}

fn query_title(select: &PatchSelect<'_>, show_play_settings_hint: bool) -> String {
    if select.drum_kit_only() {
        // `?` の help は、このモードの host（drum 画面）が selector の上に出す。
        return " Regex (空白=AND)  /:編集  Enter:kit 決定  Esc:取消  ?:help ".to_string();
    }
    let play_settings = if show_play_settings_hint {
        "  S:演奏設定"
    } else {
        ""
    };
    format!(
        " Regex (空白=AND)  /:編集  Enter:音色決定  Esc:取消  Space:試聴  m:plugin solo/mute{play_settings} "
    )
}
