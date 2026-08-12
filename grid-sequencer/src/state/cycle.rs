//! bank のダブルバッファと、次サイクルの差し替え待ち。
//!
//! chord mode は N 個のinstanceを 2 つの bank（= 2N 個の CLAP instance）へ交互に割り当てる。
//! 鳴っている bank の裏でもう一方へ次の patch を先読みしておき、進行を1周した
//! 小節境界で bank ごと差し替える。patch ロードが演奏の外側で終わっているので、
//! 差し替えに伴う無音も、鳴らしきる前に切る尻切れも起きない。
//!
//! 先読みが間に合わなかったときは差し替えを見送り、今の grid のまま次の周へ入る。
//! **音を止めないことを最優先**にするため。

use std::time::Instant;

use cmrt_realtime_play::BANK_COUNT;

use super::{ChordPlayback, GridInstance, GridState};

/// 次のサイクルへ差し替える予定の grid と進行。先読みロードが終わるまで待たせる。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PendingCycle {
    pub(super) instances: Vec<GridInstance>,
    pub(super) chord: ChordPlayback,
    /// true なら待機 bank へ先読み済みで、commit 時に bank を切り替える。
    /// patch を変えない周は false のまま現在 bank 上で取り込む。
    switch_bank: bool,
}

impl GridState {
    /// いま鳴っている bank（0 か 1）。chord mode を使わない間は 0 のまま動かない。
    pub fn bank(&self) -> usize {
        self.bank
    }

    /// 論理instanceに対応する、いま鳴らすべき CLAP instance の ID。
    pub fn instance_id(&self, instance: usize) -> u8 {
        (self.bank * self.instances.len() + instance) as u8
    }

    /// 論理instanceに対応する、待機中の bank の CLAP instance の ID。先読みロードの宛先。
    pub fn standby_instance_id(&self, instance: usize) -> u8 {
        ((self.bank + 1) % BANK_COUNT * self.instances.len() + instance) as u8
    }

    /// いま鳴っている bank の (instance ID, patch)。
    pub fn patches(&self) -> impl Iterator<Item = (u8, Option<&str>)> {
        self.instances
            .iter()
            .enumerate()
            .map(|(instance, item)| (self.instance_id(instance), item.patch.as_deref()))
    }

    /// 差し替え待ちの grid の (instance ID, patch)。先読みロードはこれを流し込む。
    /// 差し替え待ちが無ければ空。
    pub fn pending_patches(&self) -> Vec<(u8, Option<String>)> {
        let Some(pending) = self.pending.as_ref() else {
            return Vec::new();
        };
        if !pending.switch_bank {
            return Vec::new();
        }
        pending
            .instances
            .iter()
            .enumerate()
            .map(|(instance, item)| (self.standby_instance_id(instance), item.patch.clone()))
            .collect()
    }

    /// 進行の最終小節へ入ったことを一度だけ報告する。画面側が次の抽選と
    /// 先読みロードを始める合図。
    pub fn take_preload_due(&mut self) -> bool {
        std::mem::take(&mut self.preload_due)
    }

    /// 抽選し終えた次サイクルを、差し替え待ちとして預ける。
    ///
    /// この時点ではまだ鳴らさない。`mark_pending_ready()` で先読みロードの完了を
    /// 伝えたあと、進行を1周した小節境界で初めて差し替わる。
    pub fn stage_next_cycle(&mut self, instances: Vec<GridInstance>, chord: ChordPlayback) {
        debug_assert_eq!(
            instances.len(),
            self.instances.len(),
            "bank ごとのinstance数は揃える"
        );
        self.pending = Some(PendingCycle {
            instances,
            chord,
            switch_bank: true,
        });
        self.pending_ready = false;
    }

    /// patch を据え置く周のために、次の進行を現在 bank 上で差し替え待ちにする。
    ///
    /// patch の先読みは不要なので、この時点で commit 可能として扱う。実際のinstances/chord
    /// の差し替えは通常どおり進行境界まで待つ。
    pub fn stage_next_cycle_in_place(
        &mut self,
        instances: Vec<GridInstance>,
        chord: ChordPlayback,
    ) {
        debug_assert_eq!(
            instances.len(),
            self.instances.len(),
            "bank ごとのinstance数は揃える"
        );
        self.pending = Some(PendingCycle {
            instances,
            chord,
            switch_bank: false,
        });
        self.pending_ready = true;
    }

    /// 先読みロードが終わり、待機 bank が鳴らせる状態になったことを伝える。
    pub fn mark_pending_ready(&mut self) {
        self.pending_ready = self.pending.is_some();
    }

    /// 差し替え待ちがあるか。画面側の状態機械が二重に先読みを始めないために見る。
    pub fn has_pending_cycle(&self) -> bool {
        self.pending.is_some()
    }

    /// テスト用。最終小節へ入った合図を、クロックを回さずに立てる。
    #[cfg(test)]
    pub(crate) fn stage_preload_due_for_test(&mut self) {
        self.preload_due = true;
    }

    /// テスト用。差し替え待ちの grid。
    #[cfg(test)]
    pub(crate) fn pending_instances_for_test(&self) -> Vec<GridInstance> {
        self.pending
            .as_ref()
            .map(|pending| pending.instances.clone())
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn pending_rows_for_test(&self) -> Vec<GridInstance> {
        self.pending_instances_for_test()
    }

    /// テスト用。差し替え待ちのコード進行。
    #[cfg(test)]
    pub(crate) fn pending_chord_for_test(&self) -> Option<&super::ChordPlayback> {
        self.pending.as_ref().map(|pending| &pending.chord)
    }

    /// 差し替え待ちを捨てる。`r` キーのように grid を丸ごと引き直すときに使う。
    pub fn discard_pending_cycle(&mut self) {
        self.pending = None;
        self.pending_ready = false;
        self.preload_due = false;
    }

    /// サイクルを鳴らしきったらクロックを止めるようにする（シングルバッファリング）。
    /// 冪等なので毎フレーム呼んでよい。
    pub fn arm_cycle_stop(&mut self) {
        self.stop_at_cycle_end = true;
    }

    /// 鳴らしきりでの停止をやめる。ダブルバッファリングへ戻すときに呼ぶ。
    pub fn disarm_cycle_stop(&mut self) {
        self.reset_cycle_stop();
    }

    /// サイクルを鳴らしきってクロックが止まったことを一度だけ報告する。
    /// 返すのは最後の音の締切。
    pub fn take_cycle_stopped(&mut self) -> Option<Instant> {
        self.cycle_stopped_at.take()
    }

    pub(super) fn reset_cycle_stop(&mut self) {
        self.stop_at_cycle_end = false;
        self.cycle_wrapped = false;
        self.cycle_stopped_at = None;
    }

    /// bank を動かさずに差し替え待ちを取り込む（シングルバッファリング）。
    ///
    /// 待機 bank へ先読みしていないので `pending_ready` は立たない。取り込んだ patch は
    /// 呼び出し側が今の bank へロードし直す。差し替えたかどうかを返す。
    pub fn commit_pending_cycle_in_place(&mut self) -> bool {
        let Some(pending) = self.pending.take() else {
            return false;
        };
        self.instances = pending.instances;
        self.chord = Some(pending.chord);
        self.pending_ready = false;
        self.refresh_lane_display_patterns();
        true
    }

    /// 進行を1周したので、先読みが終わっていれば bank ごと差し替える。
    ///
    /// 先読みする周は、間に合っていなければ差し替えを見送って今の grid のまま次の周へ
    /// 入る。先読みしない周は同じ bank 上で進行を取り込む。
    /// 差し替えたかどうかを返す。
    pub(super) fn commit_pending_cycle(&mut self) -> bool {
        if !self.pending_ready {
            return false;
        }
        let Some(pending) = self.pending.take() else {
            return false;
        };
        self.instances = pending.instances;
        self.chord = Some(pending.chord);
        if pending.switch_bank {
            self.bank = (self.bank + 1) % BANK_COUNT;
        }
        self.pending_ready = false;
        true
    }
}

#[cfg(test)]
mod tests;
