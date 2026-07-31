//! bank のダブルバッファと、次サイクルの差し替え待ち。
//!
//! chord mode は N 個の行を 2 つの bank（= 2N 個の CLAP instance）へ交互に割り当てる。
//! 鳴っている bank の裏でもう一方へ次の patch を先読みしておき、進行を1周した
//! 小節境界で bank ごと差し替える。patch ロードが演奏の外側で終わっているので、
//! 差し替えに伴う無音も、鳴らしきる前に切る尻切れも起きない。
//!
//! 先読みが間に合わなかったときは差し替えを見送り、今の grid のまま次の周へ入る。
//! **音を止めないことを最優先**にするため。

use cmrt_realtime_play::BANK_COUNT;

use super::{ChordPlayback, GridRow, GridState};

/// 次のサイクルへ差し替える予定の grid と進行。先読みロードが終わるまで待たせる。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PendingCycle {
    pub(super) rows: Vec<GridRow>,
    pub(super) chord: ChordPlayback,
}

impl GridState {
    /// いま鳴っている bank（0 か 1）。chord mode を使わない間は 0 のまま動かない。
    pub fn bank(&self) -> usize {
        self.bank
    }

    /// 行に対応する、いま鳴らすべき CLAP instance の ID。
    pub fn instance_id(&self, row: usize) -> u8 {
        (self.bank * self.rows.len() + row) as u8
    }

    /// 行に対応する、待機中の bank の CLAP instance の ID。先読みロードの宛先。
    pub fn standby_instance_id(&self, row: usize) -> u8 {
        ((self.bank + 1) % BANK_COUNT * self.rows.len() + row) as u8
    }

    /// いま鳴っている bank の (instance ID, patch)。
    pub fn patches(&self) -> impl Iterator<Item = (u8, Option<&str>)> {
        self.rows
            .iter()
            .enumerate()
            .map(|(row, item)| (self.instance_id(row), item.patch.as_deref()))
    }

    /// 差し替え待ちの grid の (instance ID, patch)。先読みロードはこれを流し込む。
    /// 差し替え待ちが無ければ空。
    pub fn pending_patches(&self) -> Vec<(u8, Option<String>)> {
        let Some(pending) = self.pending.as_ref() else {
            return Vec::new();
        };
        pending
            .rows
            .iter()
            .enumerate()
            .map(|(row, item)| (self.standby_instance_id(row), item.patch.clone()))
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
    pub fn stage_next_cycle(&mut self, rows: Vec<GridRow>, chord: ChordPlayback) {
        debug_assert_eq!(rows.len(), self.rows.len(), "bank ごとの行数は揃える");
        self.pending = Some(PendingCycle { rows, chord });
        self.pending_ready = false;
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
    pub(crate) fn pending_rows_for_test(&self) -> Vec<GridRow> {
        self.pending
            .as_ref()
            .map(|pending| pending.rows.clone())
            .unwrap_or_default()
    }

    /// 差し替え待ちを捨てる。`r` キーのように grid を丸ごと引き直すときに使う。
    pub fn discard_pending_cycle(&mut self) {
        self.pending = None;
        self.pending_ready = false;
        self.preload_due = false;
    }

    /// 進行を1周したので、先読みが終わっていれば bank ごと差し替える。
    ///
    /// 間に合っていなければ差し替えを見送り、今の grid のまま次の周へ入る
    /// （差し替え待ちは次の周まで持ち越す）。差し替えたかどうかを返す。
    pub(super) fn commit_pending_cycle(&mut self) -> bool {
        if !self.pending_ready {
            return false;
        }
        let Some(pending) = self.pending.take() else {
            return false;
        };
        self.rows = pending.rows;
        self.chord = Some(pending.chord);
        self.bank = (self.bank + 1) % BANK_COUNT;
        self.pending_ready = false;
        true
    }
}

#[cfg(test)]
mod tests;
