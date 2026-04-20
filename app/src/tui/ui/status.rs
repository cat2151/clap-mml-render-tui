use ratatui::{
    layout::Rect,
    style::{Color, Style},
};

use super::{Mode, PlayState, TuiRenderStatus};
use crate::ui_theme::{
    MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GREEN, MONOKAI_PURPLE, MONOKAI_YELLOW,
};

pub(super) fn visible_list_page_size(area: Rect) -> usize {
    usize::from(area.height.saturating_sub(2).max(1))
}

pub(super) fn base_style() -> Style {
    Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG)
}

pub(super) fn status_color(play_state: &PlayState) -> Color {
    match play_state {
        PlayState::Err(_) => Color::Red,
        PlayState::Running(_) => MONOKAI_PURPLE,
        PlayState::Playing(_) => MONOKAI_YELLOW,
        PlayState::Done(_) => MONOKAI_GREEN,
        PlayState::Idle => MONOKAI_CYAN,
    }
}

pub(super) fn render_status_color(render_status: TuiRenderStatus) -> Color {
    if render_status.active == 0 && render_status.pending == 0 {
        MONOKAI_GREEN
    } else {
        MONOKAI_PURPLE
    }
}

pub(super) fn render_status_text(render_status: TuiRenderStatus) -> String {
    let mut text = if render_status.workers == 0 {
        format!(
            "render: 実行 {} 予約 {}",
            render_status.active, render_status.pending
        )
    } else {
        format!(
            "render: 実行 {}/{} 予約 {}",
            render_status.active, render_status.workers, render_status.pending
        )
    };
    if render_status.pending_playback > 0 {
        text.push_str(&format!(" preview待ち {}", render_status.pending_playback));
    }
    text
}

fn play_status_suffix(play_state: &PlayState) -> String {
    match play_state {
        PlayState::Idle => "".to_string(),
        PlayState::Running(mml) => format!("  ⚙ レンダリング中: {}", mml),
        PlayState::Playing(msg) => format!("  ▶ 演奏中: {}", msg),
        PlayState::Done(msg) => format!("  ✓ {}", msg),
        PlayState::Err(msg) => format!("  ✗ {}", msg),
    }
}

pub(super) fn normal_status_text(mode: &Mode, play_state: &PlayState) -> String {
    let mode = match mode {
        Mode::Insert => "INSERT",
        Mode::Help => "HELP",
        _ => "NORMAL",
    };
    format!("{mode}{}", play_status_suffix(play_state))
}

pub(super) fn notepad_mode_title(mode: &Mode) -> &'static str {
    match mode {
        Mode::Normal => " [NORMAL] notepad mode ",
        Mode::Insert => " [INSERT] notepad mode ",
        Mode::PatchSelect => " [PATCH SELECT] notepad mode ",
        Mode::NotepadHistory => " [HISTORY] notepad mode ",
        Mode::NotepadHistoryGuide => " [NORMAL] notepad mode ",
        Mode::PatchPhrase => " [PATCH PHRASE] notepad mode ",
        Mode::Help => " [HELP] notepad mode ",
    }
}

pub(super) fn keybind_text(mode: &Mode) -> &'static str {
    match mode {
        Mode::Normal => {
            "q ?:help i:insert o/O:挿入 dd/Del:cut p/P:貼付 f:phrase g:generate r:ランダム音色 t:音色 Shift+H:patch history j/k・↑↓・PgUp/PgDn・Home/M:再生移動 Enter/Space w:DAW"
        }
        Mode::Insert => "ESC:確定→NORMAL  Enter:確定→次行",
        Mode::PatchSelect => {
            "/:検索入力  Enter:検索確定/決定  Space:再生  ESC:キャンセル  Ctrl+S:sort順切替  n/p/t:overlay切替  f:お気に入り  h/l・←/→:ペイン移動  j/k・↑↓・PgUp/PgDn:移動して再生"
        }
        Mode::NotepadHistory => {
            "/:検索入力  Enter:検索確定/確定  ESC:閉じる  n/p/t:overlay切替  h/l・←/→:ペイン移動  j/k・↑↓:移動して再生  PgUp/PgDn:1画面移動  f:お気に入り  dd:削除"
        }
        Mode::NotepadHistoryGuide => "Enter:notepad history overlay  ESC:キャンセル",
        Mode::PatchPhrase => {
            "/:検索入力  Enter:検索確定/現在行の上に挿入  n/p/t:overlay切替  j/k・↑↓:再生移動  PgUp/PgDn:1画面移動  h/l・←/→:ペイン移動  Space:再生  i:編集  f:お気に入り  ESC:戻る"
        }
        Mode::Help => "ESC:キャンセル",
    }
}

pub(super) fn status_text(mode: &Mode, play_state: &PlayState) -> String {
    let play_str = play_status_suffix(play_state);
    match mode {
        Mode::Normal | Mode::Insert | Mode::NotepadHistoryGuide | Mode::Help => {
            normal_status_text(mode, play_state)
        }
        Mode::PatchSelect => format!("音色選択{}", play_str),
        Mode::NotepadHistory => format!("notepad history{}", play_str),
        Mode::PatchPhrase => format!("patch phrase{}", play_str),
    }
}
