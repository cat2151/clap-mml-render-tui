use ratatui::style::Color;

use super::{Mode, PlayState, TuiRenderStatus};
use cmrt_tui_core::theme::{MONOKAI_GREEN, MONOKAI_PURPLE};

// base_style / status_color / play_status_suffix / visible_list_page_size は画面横断で
// 共有するため `cmrt-tui-core` へ切り出した。従来の `status::*` パスは再エクスポートで維持する。
pub(super) use cmrt_tui_core::status::{
    base_style, play_status_suffix, status_color, visible_list_page_size,
};

pub(super) fn render_status_color(render_status: &TuiRenderStatus) -> Color {
    if render_status.running.is_empty() && render_status.pending.is_empty() {
        MONOKAI_GREEN
    } else {
        MONOKAI_PURPLE
    }
}

/// 例: `render実行: brass/strings  render順番待ち: guitar`。空なら `-`。
pub(super) fn render_status_text(render_status: &TuiRenderStatus) -> String {
    let names = |names: &[String]| {
        if names.is_empty() {
            "-".to_string()
        } else {
            names.join("/")
        }
    };
    format!(
        "render実行: {}  render順番待ち: {}",
        names(&render_status.running),
        names(&render_status.pending)
    )
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
        Mode::EffectChain | Mode::EffectChainAdd => " [EFFECT CHAIN] notepad mode ",
        Mode::Help => " [HELP] notepad mode ",
    }
}

pub(super) fn keybind_text(mode: &Mode) -> &'static str {
    match mode {
        Mode::Normal => {
            "q ?:help e:config b:loops i:insert o/O:挿入 dd/Del:cut p/P:貼付 f:phrase g:generate r:ランダム音色 t:音色 x:effect Shift+H:patch history j/k・↑↓・PgUp/PgDn・Home/M:再生移動 Enter/Space w:DAW v:keyboard Ctrl+G:screens"
        }
        Mode::Insert => "ESC:確定→NORMAL  Enter:確定→次行",
        Mode::PatchSelect => {
            "/:Regex検索  Enter:検索確定/決定  Space:再生  ESC:キャンセル  n/p/t:overlay切替  f:お気に入り  a:preset追加  r:ランダム  h/l・←/→:ペイン移動  j/k・↑↓・PgUp/PgDn・Home/End:移動して再生"
        }
        Mode::NotepadHistory => {
            "/:検索入力  Enter:検索確定/確定  ESC:閉じる  n/p/t:overlay切替  h/l・←/→:ペイン移動  j/k・↑↓:移動して再生  PgUp/PgDn:1画面移動  f:お気に入り  dd:削除"
        }
        Mode::NotepadHistoryGuide => "Enter:notepad history overlay  ESC:キャンセル",
        Mode::PatchPhrase => {
            "/:検索入力  Enter:検索確定/現在行の上に挿入  n/p/t:overlay切替  j/k・↑↓:再生移動  PgUp/PgDn:1画面移動  h/l・←/→:ペイン移動  Space:再生  i:編集  f:お気に入り  ESC:戻る"
        }
        Mode::EffectChain => cmrt_effect_chain_select::messages::FOOTER,
        Mode::EffectChainAdd => cmrt_effect_chain_select::messages::ADD_FOOTER,
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
        Mode::EffectChain | Mode::EffectChainAdd => format!("effect chain{}", play_str),
    }
}
