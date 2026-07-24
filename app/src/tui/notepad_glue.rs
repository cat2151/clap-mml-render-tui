//! notepad 画面と TuiApp（共有ランタイム）を接続する glue。
//!
//! 画面ロジック・描画・再生は `cmrt-notepad` crate 側にあり、ここは TuiApp の
//! フィールド（active_screen / notepad / playback_session）に触れる薄い接続層だけを残す。

use crate::tui::{Mode, TuiApp};

impl<'a> TuiApp<'a> {
    /// notepad 画面を NORMAL モードで表示中か。
    ///
    /// notepad の `Mode` は画面内のサブモードしか表さないため、`Mode::Normal` だけでは
    /// keyboard / loop browser / DAW を表示中でも真になってしまう。notepad 固有の
    /// 定期処理（音出しガイド・起動時キャッシュ）はこの判定を通すこと。
    pub(in crate::tui) fn notepad_normal_mode_active(&self) -> bool {
        self.active_screen == crate::screen_switch::PrimaryScreen::Notepad
            && self.notepad.mode == Mode::Normal
    }

    pub(in crate::tui) fn pump_notepad_sound_check_guide(&mut self) {
        let on_notepad_normal = self.notepad_normal_mode_active();
        let first_overlay_today = self.notepad.sound_check_guide_mut().tick(
            std::time::Instant::now(),
            on_notepad_normal,
            &crate::sound_check_guide::local_date_string(),
        );
        if first_overlay_today {
            if let Some(local_date) = self.notepad.sound_check_guide().last_overlay_date() {
                let _ = crate::history::save_notepad_sound_check_guide_overlay_date(local_date);
            }
        }
    }

    /// 起動後に notepad 画面を初めて表示したときの1回だけ、ディスクキャッシュを温める。
    pub(in crate::tui) fn prime_notepad_startup_cache_if_needed(&mut self, autoplay: bool) {
        if self.notepad.startup_cache_primed() || !self.notepad_normal_mode_active() {
            return;
        }
        self.notepad.prime_startup_cache(autoplay);
    }

    /// 終了・再起動・DAW への遷移など、notepad から離れる各パスで共通に行う保存処理。
    pub(in crate::tui) fn save_notepad_and_session_state(&mut self) {
        self.notepad.flush_patch_phrase_store_if_dirty();
        self.save_history_state();
        self.notepad.flush_disk_cache();
    }
}
