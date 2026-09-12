//! 音色ロードの完了を待ってから鳴らし始める段取り。
//!
//! ダブルバッファ（[`crate::cycle_swap`]）が効くのは定常運転のときだけで、画面へ入った初回と
//! `r` キーは全 instance を差し替える `prepare()`（`stop_live_all()` を伴う）が走り、無音になる。
//! この経路では `prepare()` を投げたら待ちへ入り、接続が `Ready` へ戻ってから
//! [`PATCH_SETTLE_GUARD`] だけ置いて `state.start()` でクロックを step 0 から張り直す。
//! この猶予が無いと、state をロードしただけでまだ鳴らせない Surge XT へ note on が飛び、
//! 再開の 1 打目が無音になる。サーバー側の settle（`PATCH_SETTLE_BLOCKS`）と二重の保険。

use std::time::{Duration, Instant};

use crate::GridSequencerScreen;

/// `Ready` へ戻ってから鳴らし始めるまでの猶予。サーバー側の settle だけでは足りなかった
/// ときの保険で、長くするほど再開が遅れる。
pub(crate) const PATCH_SETTLE_GUARD: Duration = Duration::from_millis(100);

impl GridSequencerScreen {
    /// `prepare()` の直後に呼び、`Ready` 復帰を待つ状態へ入る。
    ///
    /// クロック自体は止めない。待っている間 `pump_step` が `poll_steps` を呼ばず、明けた時点で
    /// `state.start()` が `now` へ張り直すため、止めても止めなくても鳴り始めは変わらない。
    pub(crate) fn wait_for_patches(&mut self) {
        self.resume_at = None;
        self.waiting_for_patches = true;
    }

    /// 毎フレーム呼ぶ。待ちが明けたらクロックを step 0 から張り直し、true を返す。
    /// 戻り値が true の間だけ [`GridSequencerScreen::pump_step`] がステップを進める。
    pub(crate) fn poll_start_wait(&mut self, now: Instant, ready: bool) -> bool {
        if !self.waiting_for_patches {
            return ready;
        }
        if !ready {
            // まだロード中。`resume_at` を立て直させる（ロードが2回続く場合がある）。
            self.resume_at = None;
            return false;
        }
        let resume_at = *self.resume_at.get_or_insert(now + PATCH_SETTLE_GUARD);
        if now < resume_at {
            return false;
        }
        self.waiting_for_patches = false;
        self.resume_at = None;
        self.restart_timeline(now);
        crate::log_line("grid-sequencer: resumed after patch load");
        true
    }
}

#[cfg(test)]
mod tests;
