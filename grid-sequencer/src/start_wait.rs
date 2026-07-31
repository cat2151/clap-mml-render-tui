//! 音色ロードの完了を待ってから鳴らし始める段取り。
//!
//! ダブルバッファ（[`crate::cycle_swap`]）が効くのは、鳴っている bank の裏で次を
//! 仕込める定常運転のときだけ。**画面へ入った初回**と **`r` キー**は全 instance を
//! 差し替えるので、`stop_live_all()` を伴う従来の `prepare()` が走り、その間は
//! どうやっても無音になる。
//!
//! この経路では:
//! 1. `prepare()` を投げたら「待ち」へ入る（`pump_step` がステップを進めなくなる）。
//! 2. 接続が `Ready` へ戻り、さらに [`PATCH_SETTLE_GUARD`] だけ待つ。
//! 3. `state.start()` でクロックを step 0 から張り直す。
//!
//! こうすると再開直後の step 0 から和音が鳴る。2 の待ちが無いと、state をロード
//! しただけでまだ鳴らせない Surge XT へ note on が飛んで最初の1打が無音になる。
//! サーバー側にも同じ目的の settle があり（`PATCH_SETTLE_BLOCKS`）、二重の保険。

use std::time::{Duration, Instant};

use crate::GridSequencerScreen;

/// `Ready` へ戻ってから鳴らし始めるまでの猶予。
///
/// サーバー側の settle（patch ロード後に空ブロックを回す）だけでは足りなかった
/// ときの保険。長くするほど再開が遅れるので、必要最小限に留める。
pub(crate) const PATCH_SETTLE_GUARD: Duration = Duration::from_millis(100);

impl GridSequencerScreen {
    /// 音色ロードを投げたあと、鳴らし始めるまで待つ。
    ///
    /// `Ready` 復帰を待つ状態へ入る。`prepare()` の直後に呼ぶこと。
    ///
    /// クロック自体は止めない。待っている間 `pump_step` が `poll_steps` を呼ばず、
    /// 待ちが明けた時点で `state.start()` が改めて `now` へ張り直すため、
    /// 止めても止めなくても鳴り始めは変わらない。
    pub(crate) fn wait_for_patches(&mut self) {
        self.resume_at = None;
        self.waiting_for_patches = true;
    }

    /// 毎フレーム呼ぶ。待ちが明けたらクロックを step 0 から張り直し、true を返す。
    ///
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
        self.state.start(now);
        crate::log_line("grid-sequencer: resumed after patch load");
        true
    }
}

#[cfg(test)]
mod tests;
