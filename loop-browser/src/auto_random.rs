//! オートランダムモード（`Shift+O`）。
//!
//! ON の間、グリッドを何周かするごとに `Shift+R` 相当の一括ランダムを自動で引き直す。
//! そのままだと差し替えのたびに準備（デコード + time stretch）を待つ無音ができるので、
//! **裏で次のグリッドを準備し、周の境目で差し替える**（ダブルバッファリング）。
//!
//! 何周で引き直すかは 1 周の演奏時間で決まる（`auto_random_cycles`）。短いグリッドは
//! 2 周演奏し、最後の 1 周を丸ごと先読みの猶予に使う。長いグリッドは 1 周だけ演奏し、
//! その 1 周の途中から先読みする（同じ内容を 2 周聴かされる間延びを避けるため）。
//!
//! 役割分担:
//! - ここ（UI スレッド）: 周回数を見て抽選し、`GridPreload` を返す。差し替わったのを検知して表示と TOML を確定する
//! - `playback::worker`: `active` を鳴らしたまま `standby` を用意し、周の境目で差し替える
//!
//! 準備が間に合わなければ worker は差し替えずに現行グリッドをもう 1 周させる。
//! そのときは stall として予約を捨て、次の周で引き直す（音を止めないことを最優先する）。

use crate::batch_random::StagedRandomGrid;
use crate::playback::position;
use crate::{LoopBrowser, LoopBrowserAction, LoopGridChange};
use cmrt_tui_core::bpm::BpmMode;

/// 1 周がこの秒数以上なら長いグリッドとみなす。
const LONG_CYCLE_SECONDS: f64 = 16.0;
/// 長いグリッドの周回数。1 周の途中から先読みするので猶予は短くなるが、
/// 16 秒あればデコード + time stretch は間に合う。
const AUTO_RANDOM_CYCLES_LONG: u64 = 1;
/// 短いグリッドの周回数。最後の 1 周が丸ごと先読みの猶予になる。
const AUTO_RANDOM_CYCLES_SHORT: u64 = 2;
/// 予約してからこの周回数だけ待っても差し替わらなければ、諦めて引き直す。
const AUTO_RANDOM_STALL_CYCLES: u64 = 2;

/// 抽選済みで、まだ鳴り始めていないグリッド。
pub struct StagedAutoRandom {
    token: u64,
    staged_at_cycle: u64,
    staged: StagedRandomGrid,
}

impl LoopBrowser {
    pub fn auto_random(&self) -> bool {
        self.metadata.value.auto_random
    }

    /// いま鳴っているグリッドを何周したら引き直すか。1 周が長いほど少ない周回で次へ行く。
    ///
    /// 見るのは `self.track_grid`（＝いま鳴っているグリッド）。auto random は差し替わってから
    /// commit するので、抽選する時点の `track_grid` は再生中のものと一致する。
    pub fn auto_random_cycles(&self) -> u64 {
        if self.cycle_seconds() >= LONG_CYCLE_SECONDS {
            AUTO_RANDOM_CYCLES_LONG
        } else {
            AUTO_RANDOM_CYCLES_SHORT
        }
    }

    pub fn toggle_auto_random(&mut self) {
        if !self.metadata.writable {
            return;
        }
        if self
            .metadata
            .try_mutate(
                |metadata| metadata.toggle_auto_random(),
                |path, metadata| metadata.save_to(path),
                "auto randomの設定を保存できません",
            )
            .is_none()
        {
            return;
        }
        // OFF にしたら予約中の抽選は破棄する。ON に戻したときは次の周から数え直す。
        if !self.auto_random() {
            self.auto_random_staged = None;
        }
    }

    /// 毎フレーム呼ばれる。周回数を見て「確定 → stall 判定 → 抽選」の順に進める。
    pub fn pump_auto_random(&mut self) -> LoopBrowserAction {
        let Some((cycle, token)) = position::current_grid(&self.playback_position) else {
            return LoopBrowserAction::Continue;
        };
        if self.commit_staged_auto_random(token) {
            return LoopBrowserAction::Continue;
        }
        self.discard_stalled_auto_random(cycle);
        self.stage_auto_random(cycle)
    }

    /// 予約したグリッドが鳴り始めていれば、そこで初めて表示と TOML を確定する。
    fn commit_staged_auto_random(&mut self, playing_token: u64) -> bool {
        let Some(staged) = &self.auto_random_staged else {
            return false;
        };
        if staged.token != playing_token {
            return false;
        }
        let staged = self
            .auto_random_staged
            .take()
            .expect("直前に Some を確認済み");
        self.commit_random_grid(staged.staged);
        true
    }

    fn discard_stalled_auto_random(&mut self, cycle: u64) {
        let stalled = self.auto_random_staged.as_ref().is_some_and(|staged| {
            cycle
                >= staged
                    .staged_at_cycle
                    .saturating_add(AUTO_RANDOM_STALL_CYCLES)
        });
        if stalled {
            self.auto_random_staged = None;
        }
    }

    fn stage_auto_random(&mut self, cycle: u64) -> LoopBrowserAction {
        if !self.auto_random()
            || self.auto_random_staged.is_some()
            || cycle.saturating_add(1) < self.auto_random_cycles()
        {
            return LoopBrowserAction::Continue;
        }
        // 周の区切りは自動BPMを引き直す区切りでもある。範囲に幅が無ければ据え置き。
        let Some(staged) = self.stage_random_grid(self.next_auto_bpm_mode()) else {
            return LoopBrowserAction::Continue;
        };
        self.auto_random_next_token = self.auto_random_next_token.wrapping_add(1).max(1);
        let token = self.auto_random_next_token;
        let grid = self.playback_grid_of(&staged.grid);
        let mode = staged.mode;
        self.auto_random_staged = Some(StagedAutoRandom {
            token,
            staged_at_cycle: cycle,
            staged,
        });
        LoopBrowserAction::GridPreload {
            grid,
            token,
            mode,
            reason: LoopGridChange::AutoRandom,
        }
    }

    /// 次の周へ向けて引き直した自動モード。手動モード中と、幅のない範囲では据え置き。
    fn next_auto_bpm_mode(&self) -> BpmMode {
        if self.bpm_mode.auto_target().is_none() || self.bpm_range.is_fixed() {
            return self.bpm_mode;
        }
        BpmMode::Auto(self.bpm_range.sample())
    }
}
