//! 移動・音長・velocity・pattern 切替は repeat を許可し、セル切替と help の開閉と数字は Press の 1 回だけ受け付ける。
//! 上下は画面上の向き。高い note を上に描くので、上へ動くと note が高くなる。
//! vim 風に数字を前置すると、次のキーをその回数ぶん繰り返す。Space と `?` は回数を捨てる。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{DrumSequencerScreen, DRUM_STEPS};

/// `,` / `.` 1 回で変える velocity。127 から 16 回で 1 に届く。
const VELOCITY_STEP: i8 = 8;
/// 前置できる回数の上限。行数（最大 128）を越えれば足りる。
const MAX_COUNT: usize = 999;

impl DrumSequencerScreen {
    /// matrix のキーを処理したかを返す。selector の入力は host が先に消費する。
    /// help 表示中は全キーを消費し、`?` / Esc でだけ閉じる。q で閉じると、続けて押した q がアプリ終了になる。
    /// kit 未選択・不明・空一覧では、入力を変更しない。pattern の切替はできる。
    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        if self.help_open {
            if key.kind == KeyEventKind::Press
                && matches!(key.code, KeyCode::Char('?') | KeyCode::Esc)
            {
                self.help_open = false;
            }
            return true;
        }
        if key.kind == KeyEventKind::Release
            || key.modifiers.intersects(
                KeyModifiers::CONTROL
                    | KeyModifiers::ALT
                    | KeyModifiers::SUPER
                    | KeyModifiers::HYPER
                    | KeyModifiers::META,
            )
        {
            return false;
        }
        if let KeyCode::Char(digit @ '0'..='9') = key.code {
            // 先頭の 0 は回数にならない。
            if digit != '0' || self.count.is_some() {
                if key.kind == KeyEventKind::Press {
                    let value = self.count.unwrap_or(0) * 10 + (digit as usize - '0' as usize);
                    self.count = Some(value.min(MAX_COUNT));
                }
                return true;
            }
        }
        let count = self.count.take().unwrap_or(1);
        if key.code == KeyCode::Char('?') {
            if key.kind == KeyEventKind::Press {
                self.help_open = true;
            }
            return true;
        }
        match key.code {
            KeyCode::Char('[') => {
                self.shift_pattern(-(count as isize));
                return true;
            }
            KeyCode::Char(']') => {
                self.shift_pattern(count as isize);
                return true;
            }
            _ => {}
        }
        let handled = matches!(
            key.code,
            KeyCode::Char('h' | 'j' | 'k' | 'l' | ' ' | '-' | '+' | '=' | ',' | '.')
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Enter
        );
        if !handled || self.notes().is_empty() {
            return handled;
        }
        // 音長は 16、velocity は 127 で止まるので、i8 に収まらない回数は丸めてよい。
        let times = i8::try_from(count).unwrap_or(i8::MAX);
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                self.cursor_step = self.cursor_step.saturating_sub(count);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cursor_step = (self.cursor_step + count).min(DRUM_STEPS - 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.cursor_row = (self.cursor_row + count).min(self.notes().len() - 1);
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.cursor_row = self.cursor_row.saturating_sub(count);
            }
            KeyCode::Char(' ') if key.kind == KeyEventKind::Press => self.toggle_cursor(),
            KeyCode::Enter if key.kind == KeyEventKind::Press => self.toggle_rightward(count),
            KeyCode::Char('-') => self.adjust_cursor_length(-times),
            KeyCode::Char('+' | '=') => self.adjust_cursor_length(times),
            KeyCode::Char(',') => self.adjust_cursor_velocity(-VELOCITY_STEP.saturating_mul(times)),
            KeyCode::Char('.') => self.adjust_cursor_velocity(VELOCITY_STEP.saturating_mul(times)),
            _ => {}
        }
        handled
    }

    /// 前置して、まだ消費していない回数。
    pub fn pending_count(&self) -> Option<usize> {
        self.count
    }

    /// 前置した数字を捨てる。host が matrix へ渡さずに消費したキー（kit 選択など）で呼ぶ。
    pub fn clear_count(&mut self) {
        self.count = None;
    }

    /// Enter。「Space のち l」を `count` 回。右端のセルを切り替えたら止まり、同じセルを戻さない。
    fn toggle_rightward(&mut self, count: usize) {
        for _ in 0..count {
            self.toggle_cursor();
            if self.cursor_step == DRUM_STEPS - 1 {
                break;
            }
            self.cursor_step += 1;
        }
    }
}

#[cfg(test)]
mod tests;
