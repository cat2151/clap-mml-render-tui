//! MML 入力オーバーレイと共有ランタイムの接続。
//!
//! オーバーレイ自体は `cmrt-mml-overlay` crate に閉じている。ここが持つのは
//! 「どの画面から開いてよいか」「開くときに何を止めるか」「鳴らす先はどこか」
//! 「開くときに何のスナップショットを渡すか」の 4 つだけ。

use std::time::Instant;

use crossterm::event::KeyEvent;

use super::mml_overlay::{
    is_mml_overlay_trigger, MmlOverlayAction, MmlOverlayContext, PatchChange,
};
use super::{PatchLoadState, TuiApp};

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
        let context = self.mml_overlay_context();
        self.mml_overlay.open(context);
        if let Some(sender) = &self.mml_overlay_sender {
            sender.prepare(self.mml_overlay.patch());
        }
        true
    }

    /// 音色一覧とフレーズ履歴は、開くたびに最新のスナップショットを渡す。
    /// 起動直後で読み込みが終わっていなければ空のまま開き、その選択だけが効かない。
    fn mml_overlay_context(&self) -> MmlOverlayContext {
        let patches = self.loaded_patch_pairs();
        let (history, favorites) = self.notepad.phrase_history();
        MmlOverlayContext {
            patches,
            history: history.to_vec(),
            favorites: favorites.to_vec(),
        }
    }

    pub(in crate::tui) fn loaded_patch_pairs(&self) -> Vec<(String, String)> {
        match &*self.patch_load_state.lock().unwrap() {
            PatchLoadState::Ready(pairs) => pairs.clone(),
            PatchLoadState::Loading | PatchLoadState::Err(_) => Vec::new(),
        }
    }

    /// オーバーレイが開いている間、キーはすべてオーバーレイが取る。
    pub(in crate::tui) fn handle_mml_overlay_key_event(&mut self, key: KeyEvent) {
        let action = self.mml_overlay.handle_key(key, Instant::now());
        self.apply_mml_overlay_action(action);
    }

    /// 鳴らした音の gate が切れていれば止める。毎フレーム呼ぶ。
    pub(in crate::tui) fn pump_mml_overlay(&mut self) {
        if let Some(action) = self.mml_overlay.poll(Instant::now()) {
            self.apply_mml_overlay_action(action);
        }
    }

    /// オーバーレイの求めを sender へ流す。
    ///
    /// note off はここには出てこない。「鳴っているものを止める」は sender 側が
    /// 1 か所で持っていて、音を鳴らすコマンドはどれも停止込みの意味になっている。
    fn apply_mml_overlay_action(&mut self, action: MmlOverlayAction) {
        // 閉じるときだけ sender の外へ用がある（音源を借りていた画面へ返す）ので、
        // sender を借りる前に片づける。
        if action == MmlOverlayAction::Close {
            if let Some(sender) = &self.mml_overlay_sender {
                sender.stop();
            }
            // 借りていた音源を返す。開いたときに止めた演奏はここで戻る。
            self.resume_active_screen_playback();
            return;
        }
        let Some(sender) = &self.mml_overlay_sender else {
            return;
        };
        match action {
            MmlOverlayAction::Continue | MmlOverlayAction::Close => {}
            MmlOverlayAction::Send(messages) => sender.send(messages),
            MmlOverlayAction::SetPatch { patch, messages } => {
                // 音色の読み込みと発音は同じワーカースレッドが順に処理するので、
                // ここで積む順序がそのまま音源へ届く順序になる。
                sender.prepare(patch.as_deref());
                sender.send(messages);
            }
            MmlOverlayAction::PlayLine { patch, events } => {
                if let PatchChange::Switch(patch) = patch {
                    sender.prepare(patch.as_deref());
                }
                sender.play_line(events);
            }
            MmlOverlayAction::Stop => sender.stop(),
        }
    }
}
