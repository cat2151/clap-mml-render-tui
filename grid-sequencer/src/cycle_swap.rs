//! 進行1周ごとの bank 差し替えを、演奏を止めずに進める段取り。
//!
//! ドメイン側（[`crate::state::cycle`]）が「差し替え待ちをどう保持し、いつ差し替えるか」を
//! 持つのに対し、ここは「いつ次を抽選し、待機 bank へどの順で patch を流し込むか」を持つ。
//!
//! 流れ:
//! 1. 進行の先頭が実際に鳴り始めると `GridState` が `preload_due` を立てる。
//! 2. [`crate::CycleRandom`] に従って次サイクルを抽選し、差し替え待ちとして預ける。
//! 3. PATCH が ON の周だけ、**1ステップにつき1インスタンスずつ**待機 bank へ patch を
//!    先読みする。ロード中も演奏 bank の render はサーバー側で回り続け、
//!    [`crate::sender`] の送信スレッドも止まらない（受付だけして完了は状態として待つ）。
//!    OFF の周は音色が変わらないので、現在 bank 上で取り込むだけで済む。
//! 4. 全件終わったら「準備できた」と伝える。次に進行を1周した小節境界で差し替わる。
//!
//! 小節の頭までに間に合わなければ差し替えを見送り、今の grid のまま次の周へ入る。
//! **音を止めないことを最優先**にする。

use std::time::Instant;

use cmrt_tui_core::patch_load::PatchLoadMeasurement;

use crate::{log_line, GridSequencerContext, GridSequencerScreen};

/// catalog値を持たないpatchだけが選ばれた場合の控えめな初期値。1件目の実測後は
/// その値で補正されるので、未知のまま固定されることはない。
const DEFAULT_PATCH_LOAD_MS: u64 = 100;

/// 待機 bank への先読みロードの進み具合。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CycleSwap {
    /// 次に先読みするinstance。instance数に達したら送信完了。
    next_instance: usize,
    /// 送信し終えた patch の件数（= 完了を待っている件数）。
    sent: usize,
    /// 直前に送った時刻。送信間隔を空けるために見る。
    last_sent: Option<Instant>,
}

impl CycleSwap {
    fn new() -> Self {
        Self {
            next_instance: 0,
            sent: 0,
            last_sent: None,
        }
    }

    /// いま次の1件を送ってよいか。
    ///
    /// `pump_step` は**ステップごとではなく UI のフレームごと**に呼ばれるので、
    /// 何もしないと 16 件が数百 ms のうちに全部キューへ積まれる。送信スレッドは
    /// もうロードの完了を待たないので**塞がりはしない**が、まとめて投げても
    /// 早くはならない。完了通知の slot は共有メモリ上に1件ぶんしか無く、
    /// サーバー側も2件目の受付を断るので、受付の順番待ちが積み上がるだけになる。
    /// 進捗と ETA も「いま何件目か」を前提にしているので、2つの条件で間隔を空ける:
    /// - 直前の1件がサーバー側で終わっている（同時に走らせない。完了通知 slot は
    ///   一度に1件しか持てない）
    /// - 前回の送信から1ステップぶん空いている
    fn may_send(&self, now: Instant, completed: usize, interval: std::time::Duration) -> bool {
        if completed < self.sent {
            return false;
        }
        match self.last_sent {
            Some(last) => now.saturating_duration_since(last) >= interval,
            None => true,
        }
    }
}

impl GridSequencerScreen {
    /// 毎フレーム呼ぶ。先読みの開始・1件ずつの送信・完了判定をここで進める。
    pub(crate) fn advance_cycle_swap(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        if self.state.take_preload_due() {
            self.begin_cycle_swap(now, ctx);
        }
        let Some(swap) = self.cycle_swap.clone() else {
            return;
        };
        let (completed, failed) = match &self.midi_sender {
            Some(_) => {
                let status = self.connection_status();
                (status.preload.completed, status.preload_failed)
            }
            // MIDI を送らないテストモードでは待つ相手がいない。待たせると bank が
            // 永久に切り替わらなくなるので、送った時点で完了とみなす。
            None => (swap.sent, false),
        };
        if swap.next_instance < self.state.instance_count() {
            if swap.may_send(now, completed, crate::step_interval_at(self.bpm())) {
                self.preload_one_instance(swap.next_instance, now);
            }
            return;
        }
        // 全件送り終えた。最後の1件がサーバー側で終わるのを待つ。
        if completed < swap.sent {
            return;
        }
        self.finish_cycle_swap(failed);
    }

    /// 待機 bank の1行ぶんだけ patch を先読みする。
    fn preload_one_instance(&mut self, instance: usize, now: Instant) {
        let (instance_id, patch) = self
            .state
            .pending_patches()
            .get(instance)
            .cloned()
            .unwrap_or((self.state.standby_instance_id(instance), None));
        if let Some(sender) = &self.midi_sender {
            sender.preload(instance_id, patch.as_deref());
        }
        if let Some(swap) = self.cycle_swap.as_mut() {
            swap.next_instance += 1;
            swap.sent += 1;
            swap.last_sent = Some(now);
        }
    }

    /// 次サイクルを抽選し、差し替え待ちとして預けて先読みを始める。
    ///
    /// 抽選に失敗したら差し替えそのものを見送る（今の進行がもう1周する）。
    fn begin_cycle_swap(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        if self.cycle_swap.is_some() || self.state.has_pending_cycle() {
            // 前回の先読みがまだ終わっていない。追い越さない。
            return;
        }
        if !self.stage_next_cycle(now, ctx) {
            return;
        }
        // 音色が変わらない周は、待機 bank への先読みも bank 切替も要らない。
        // 差し替えは現在 bank 上で境界時に取り込まれる（`stage_next_cycle_in_place`）。
        if !self.cycle_random.patch {
            log_line(&format!(
                "grid-sequencer: in-place cycle ready active_bank={} preload=skipped random={}",
                self.state.bank(),
                self.cycle_random.compact_label(),
            ));
            return;
        }
        if let Some(sender) = &self.midi_sender {
            let pending = self.state.pending_patches();
            sender.begin_preload_cycle(preload_weights_ms(&pending, ctx.load_measurements));
        }
        self.cycle_swap = Some(CycleSwap::new());
        log_line(&format!(
            "grid-sequencer: preload begin standby_bank={} instances={}",
            (self.state.bank() + 1) % cmrt_realtime_play::BANK_COUNT,
            self.state.instance_count(),
        ));
    }

    /// 先読みを終える。成功していれば次の小節境界で差し替わる。
    fn finish_cycle_swap(&mut self, failed: bool) {
        self.cycle_swap = None;
        if let Some(sender) = &self.midi_sender {
            sender.finish_preload();
        }
        if failed {
            // 1件でも失敗した bank へ切り替えると、その行が無音になる。
            // 差し替え待ちを捨て、今の grid のまま演奏を続ける。
            self.state.discard_pending_cycle();
            log_line("grid-sequencer: preload aborted, keeping the current bank");
            return;
        }
        self.state.mark_pending_ready();
        log_line("grid-sequencer: preload ready");
    }

    /// 先読みを打ち切る。`r` キーのように grid を丸ごと差し替えるときに呼ぶ。
    ///
    /// シングルバッファリング（[`crate::single_buffer`]）の待ちも一緒に畳む。どちらの
    /// 段取りも「次サイクルへの差し替え待ち」なので、捨てるときは足並みを揃える。
    pub(crate) fn cancel_cycle_swap(&mut self) {
        let restart_preload = self.state.is_running();
        if self.cycle_swap.take().is_some() {
            if let Some(sender) = &self.midi_sender {
                sender.finish_preload();
            }
        }
        self.cycle_end_at = None;
        self.state.disarm_cycle_stop();
        self.state.discard_pending_cycle();
        // auto random が有効なままなら、手編集や設定変更で古い先読みを捨てても
        // 次の境界まで待たず、現在の内容から次サイクルを仕込み直す。single-buffer の
        // drain 待ちで既に停止している場合は、現在譜面を再開する段取りを保つ。
        if restart_preload {
            self.state.request_cycle_preload();
        }
    }

    /// live edit でpending instancesだけを捨てる。
    ///
    /// シングルバッファリングが既にサイクルを鳴らしきっていた場合、出力リングの
    /// drain 時刻は残す。これまで一緒に消すと、停止済み clock を再開する契機まで
    /// 失われるため。
    pub(crate) fn cancel_cycle_swap_preserving_drain(&mut self) {
        let drain_end = (self.single_buffering && !self.state.is_running())
            .then_some(self.cycle_end_at)
            .flatten();
        self.cancel_cycle_swap();
        self.cycle_end_at = drain_end;
    }
}

/// 今回選ばれた patch 群の catalog 時間を、順序を保ったまま ETA の重みにする。
/// 欠測値には今回の既知値の中央値（それも無ければ catalog 全体の中央値）を使う。
/// `None` は patch の解除なので、最小の重みだけを与える。
fn preload_weights_ms(
    patches: &[(u8, Option<String>)],
    measurements: Option<&std::collections::BTreeMap<String, PatchLoadMeasurement>>,
) -> Vec<u64> {
    let selected_known = patches
        .iter()
        .filter_map(|(_, patch)| measured_ms(measurements, patch.as_deref()))
        .collect::<Vec<_>>();
    let fallback = median(selected_known).or_else(|| {
        measurements.map(|measurements| {
            median(
                measurements
                    .values()
                    .filter_map(|measurement| measurement.second_load_ms)
                    .map(|ms| ms.max(1))
                    .collect(),
            )
            .unwrap_or(DEFAULT_PATCH_LOAD_MS)
        })
    });
    let fallback = fallback.unwrap_or(DEFAULT_PATCH_LOAD_MS);
    patches
        .iter()
        .map(|(_, patch)| match patch.as_deref() {
            Some(patch) => measured_ms(measurements, Some(patch)).unwrap_or(fallback),
            None => 1,
        })
        .collect()
}

fn measured_ms(
    measurements: Option<&std::collections::BTreeMap<String, PatchLoadMeasurement>>,
    patch: Option<&str>,
) -> Option<u64> {
    measurements?
        .get(patch?)?
        .second_load_ms
        .map(|ms| ms.max(1))
}

fn median(mut values: Vec<u64>) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        let lower = values[middle - 1];
        Some(lower.saturating_add((values[middle] - lower) / 2))
    } else {
        Some(values[middle])
    }
}

#[cfg(test)]
mod tests;
