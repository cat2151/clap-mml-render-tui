//! テストユーティリティ。
//!
//! history / config / ログのパスをテスト専用ディレクトリへ差し替えるヘルパは、
//! 永続化層と同じ場所にある方が壊れにくいため `cmrt-history` の `test_support` へ
//! 切り出した。従来の `crate::test_utils::*` パスは再エクスポートで維持する。
//! ここに残すのは ratatui バッファを読む app 固有のヘルパだけ。

use std::path::Path;

use ratatui::buffer::Buffer;

pub(crate) use cmrt_history::test_support::{
    session_state_path_for_test, set_local_dir_envs, test_app_dir_for_current_thread_or_default,
    TestEnvGuard,
};

/// 旧ヘルパ名を使っているテスト向けの後方互換ラッパ。
#[allow(dead_code)]
pub(crate) fn set_data_local_dir_envs(base: &Path) -> TestEnvGuard {
    set_local_dir_envs(base)
}

pub(crate) fn find_text_ignoring_spaces(buffer: &Buffer, text: &str) -> (u16, u16) {
    for y in 0..buffer.area.height {
        let mut normalized = String::new();
        let mut x_positions = Vec::new();
        for x in 0..buffer.area.width {
            let symbol = buffer
                .cell((x, y))
                .unwrap_or_else(|| panic!("failed to access buffer cell at ({x}, {y})"))
                .symbol();
            if symbol == " " || symbol.is_empty() {
                continue;
            }
            for ch in symbol.chars() {
                normalized.push(ch);
                x_positions.push(x);
            }
        }
        if let Some(byte_index) = normalized.find(text) {
            let char_index = normalized[..byte_index].chars().count();
            return (x_positions[char_index], y);
        }
    }
    panic!("text not found in buffer when ignoring spaces: {text}");
}

pub(crate) fn help_overlay_bounds(buffer: &Buffer) -> (u16, u16, u16, u16) {
    let (title_x, top) = find_text_ignoring_spaces(buffer, "ヘルプ(Keybinds)");

    let mut left = title_x;
    while left > 0
        && buffer
            .cell((left, top))
            .unwrap_or_else(|| panic!("failed to access buffer cell at ({left}, {top})"))
            .symbol()
            != "┌"
    {
        left -= 1;
    }

    let mut right = title_x;
    while right + 1 < buffer.area.width
        && buffer
            .cell((right, top))
            .unwrap_or_else(|| panic!("failed to access buffer cell at ({right}, {top})"))
            .symbol()
            != "┐"
    {
        right += 1;
    }

    let mut bottom = top;
    while bottom + 1 < buffer.area.height {
        if buffer
            .cell((left, bottom))
            .unwrap_or_else(|| panic!("failed to access buffer cell at ({left}, {bottom})"))
            .symbol()
            == "└"
            && buffer
                .cell((right, bottom))
                .unwrap_or_else(|| panic!("failed to access buffer cell at ({right}, {bottom})"))
                .symbol()
                == "┘"
        {
            break;
        }
        bottom += 1;
    }

    (left, top, right, bottom)
}
