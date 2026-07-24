//! ratatui バッファを検査する UI テストヘルパ（画面横断で共有）。
//!
//! `test-support` feature（または `cfg(test)`）でのみ有効。app / DAW 等のテストが
//! 描画結果バッファから文字位置やオーバーレイ枠を探すのに使う。

use ratatui::buffer::Buffer;

/// バッファ内から `text` を空白無視で探し、その先頭文字の (x, y) を返す。見つからなければ panic。
pub fn find_text_ignoring_spaces(buffer: &Buffer, text: &str) -> (u16, u16) {
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

/// ヘルプオーバーレイの枠 (left, top, right, bottom) を返す。
pub fn help_overlay_bounds(buffer: &Buffer) -> (u16, u16, u16, u16) {
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
