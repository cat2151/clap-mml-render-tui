//! MML 入力オーバーレイと共有ランタイムの接続。
//!
//! オーバーレイ自体は `cmrt-mml-overlay` crate に閉じている。ここが持つのは
//! 「どの画面から開いてよいか」「開くときに何を止めるか」「鳴らす先はどこか」の 3 つだけ。

use std::time::Instant;

use crossterm::event::KeyEvent;

use super::mml_overlay::{is_mml_overlay_trigger, MmlOverlayAction};
use super::TuiApp;

impl TuiApp<'_> {
    /// Ctrl+P ならオーバーレイを開く。開いたら true。
    ///
    /// 開ける条件は画面切替メニューと同じにしてある。どちらも「いまの画面が
    /// モーダルな入力中でないこと」を求めるため。
    pub(in crate::tui) fn try_open_mml_overlay(&mut self, key: KeyEvent) -> bool {
        if !is_mml_overlay_trigger(key) || !self.can_open_screen_switch_menu() {
            return false;
        }
        // オーバーレイは keyboard 画面と同じ音源インスタンスを借りるので、
        // 先にいまの画面の演奏を止めて明け渡してもらう。
        self.stop_active_screen_playback();
        self.mml_overlay.open();
        if let Some(sender) = &self.mml_overlay_sender {
            sender.prepare();
        }
        true
    }

    /// オーバーレイが開いている間、キーはすべてオーバーレイが取る。
    pub(in crate::tui) fn handle_mml_overlay_key_event(&mut self, key: KeyEvent) {
        match self.mml_overlay.handle_key(key, Instant::now()) {
            MmlOverlayAction::Continue => {}
            MmlOverlayAction::Send(messages) => self.send_mml_overlay_messages(messages),
            MmlOverlayAction::Close(messages) => {
                self.send_mml_overlay_messages(messages);
                // 借りていた音源を返す。開いたときに止めた演奏はここで戻る。
                self.resume_active_screen_playback();
            }
        }
    }

    /// 鳴らした音の gate が切れていれば止める。毎フレーム呼ぶ。
    pub(in crate::tui) fn pump_mml_overlay(&mut self) {
        if let Some(messages) = self.mml_overlay.poll(Instant::now()) {
            self.send_mml_overlay_messages(messages);
        }
    }

    fn send_mml_overlay_messages(&self, messages: Vec<[u8; 3]>) {
        if let Some(sender) = &self.mml_overlay_sender {
            sender.send(messages);
        }
    }
}
