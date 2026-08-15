use std::io::{self, Write};

use crossterm::{
    execute,
    terminal::{DisableLineWrap, EnableLineWrap},
};

/// TUI が右下セルへ描画しても端末自体をスクロールさせない。
pub(super) fn disable<W: Write>(writer: &mut W) -> io::Result<()> {
    execute!(writer, DisableLineWrap)
}

/// 呼び出し元のシェルや外部エディタへ端末を返す前に既定動作へ戻す。
pub(super) fn enable<W: Write>(writer: &mut W) -> io::Result<()> {
    execute!(writer, EnableLineWrap)
}

#[cfg(test)]
mod tests;
