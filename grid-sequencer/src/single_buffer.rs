//! 裏読みをやめ、鳴らしきってから音色をロードするシングルバッファリング。
//!
//! ダブルバッファリング（[`crate::cycle_swap`]）は、演奏の裏で待機 bank へ patch を
//! 仕込めるだけの余裕がサーバー側にあることを前提にしている。16 track のようにその
//! 前提が崩れる環境では、出力バッファを上限まで厚くしてもフレームドロップが止まらず、
//! 鳴っている最中にブツ切れが続く。
//!
//! そこを [`crate::sender::overload`] が検出したら（`b` キーでも手動で切り替えられる）、
//! 演奏とロードを競合させるのをやめ、サイクルごとに次を順番に行う:
//! 1. 進行を1周ぶん鳴らしきる（[`crate::state`] がクロックを畳む）。
//! 2. 出力リングに溜まった音が出終わるまで待つ。
//! 3. 今の bank へ音色をロードし、完了まで待つ（[`crate::start_wait`]）。
//! 4. 先頭から演奏を再開する。
//!
//! 2 が要るのは、ロードに伴う `stop_live_all()` がサーバー側で世代を進め、**リングに
//! 溜まった描画済みの音声を捨てる**ため。ここを待たないと最後の小節が尻切れになる。

use std::time::{Duration, Instant};

use crate::{log_line, GridSequencerContext, GridSequencerScreen};

/// 出力リングを吐き出しきるまでの上乗せ。見積もりが少し足りなくても切れないようにする。
const DRAIN_MARGIN: Duration = Duration::from_millis(50);

impl GridSequencerScreen {
    /// シングルバッファリングへ落ちる。走っている先読みは意味を失うので捨てる。
    pub(crate) fn enter_single_buffering(&mut self, reason: &str) {
        self.overload_applied = true;
        if self.single_buffering {
            return;
        }
        self.cancel_cycle_swap();
        self.single_buffering = true;
        log_line(&format!(
            "grid-sequencer: single buffering on reason={reason}"
        ));
    }

    /// `b` キー。検証用に手動で切り替える。
    ///
    /// 手動で戻したあと自動判定に蒸し返されないよう、`overload_applied` は下ろさない。
    pub(crate) fn toggle_single_buffering(&mut self) {
        if self.single_buffering {
            self.cancel_cycle_swap();
            self.single_buffering = false;
            self.overload_applied = true;
            log_line("grid-sequencer: single buffering off reason=manual");
            return;
        }
        self.enter_single_buffering("manual");
    }

    /// 毎フレーム呼ぶ。鳴らしきり → 吐き出し待ち → 音色ロード、を進める。
    pub(crate) fn advance_single_buffer_cycle(
        &mut self,
        now: Instant,
        ctx: &GridSequencerContext<'_>,
    ) {
        // 進行が無ければサイクルの区切りも無い。従来どおり鳴らし続ける。
        if self.state.chord().is_none() {
            return;
        }
        self.state.arm_cycle_stop();
        // 進行の先頭から次サイクルを抽選しておく。ロードはまだ走らせない。
        if self.state.take_preload_due() && !self.state.has_pending_cycle() {
            self.stage_next_cycle(now, ctx);
        }
        if let Some(deadline) = self.state.take_cycle_stopped() {
            let drain = self.output_drain();
            self.cycle_end_at = Some(deadline + drain);
            log_line(&format!(
                "grid-sequencer: cycle played out, draining for {}ms",
                drain.as_millis()
            ));
        }
        let Some(load_at) = self.cycle_end_at else {
            return;
        };
        if now < load_at {
            return;
        }
        self.cycle_end_at = None;
        if !self.state.commit_pending_cycle_in_place() {
            // 抽選できていない。音色が変わらないのでロードせず、そのまま次の周へ入る。
            log_line("grid-sequencer: no staged cycle, restarting without a patch load");
            self.restart_timeline(now);
            return;
        }
        self.prepare_connection();
    }

    /// 出力リングに溜まった音が鳴り終わるまでの見込み時間。
    ///
    /// 実際のリング残量は共有メモリに出ていないので、目標レイテンシで多めに見積もる。
    /// 足りないと最後の小節が切れるが、多い側は無音が延びるだけで済む。
    fn output_drain(&self) -> Duration {
        let multiplier = self.connection_status().buffer_multiplier;
        Duration::from_secs_f64(self.buffer_latency_ms(multiplier) / 1000.0) + DRAIN_MARGIN
    }
}

#[cfg(test)]
mod tests;
