//! 演奏を止めずにテンポを乗り換える経路。
//!
//! テンポ変更は「クロックを作り直す」ことではなく「いつから何 BPM か」というデータの
//! 追記として扱う。境目のステップは変更前のテンポで決まった位置のまま鳴り、次の
//! ステップから新しい間隔になるので、wall clock も絶対 musical time も連続する。
//!
//! 乗り換えた結果は [`AppliedTempo`] で画面側へ渡す。画面側はそれを表示へ反映し、
//! 同じ変化点をサーバーの tempo map へも積む（[`crate::tempo`]）。

use super::GridState;

/// 実際に乗り換えたテンポと、その境目の絶対 musical time。
///
/// 秒はそのままサーバーの tempo map へ積む「この秒から新テンポ」になる。BPM だけを
/// 持ち回ると「いつから」がサーバーへ伝わらず、CLAP transport のテンポが追従しない。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppliedTempo {
    pub bpm: f64,
    pub at_timeline_seconds: f64,
}

impl GridState {
    /// 予約されたテンポへ、コード進行1周の頭から乗り換える。
    ///
    /// 呼び出し元（`advance_schedule`）が「進行が先頭のコードへ戻った小節境界か」を
    /// 判定して呼ぶ。小節ごとに呼ぶと進行の途中でテンポが動いてフレーズが繋がらない。
    ///
    /// テンポは `step` の締切をアンカーに張り替えるので、`step` 自身はこれまでどおりの
    /// 位置で鳴り、次のステップから新しい間隔になる。演奏も絶対 musical time も
    /// 途切れないため、サーバー側の timeline を張り直す必要が無い。
    pub(super) fn apply_next_cycle_bpm(&mut self, step: u64) {
        let Some(bpm) = self.next_cycle_bpm.take() else {
            return;
        };
        if bpm == self.clock.bpm() {
            return;
        }
        let Some(at_timeline_seconds) = self.clock.retempo(step, bpm) else {
            return;
        };
        self.applied_cycle_bpm = Some(AppliedTempo {
            bpm,
            at_timeline_seconds,
        });
    }

    /// 演奏中に、まだ組み立てていない次のステップからテンポを乗り換える。
    ///
    /// 先読み済み（送信済み）のステップは変更前のテンポで確定しているので、境目は
    /// 必ず `next_step` にする。手動 BPM 変更用で、周の予約とは別経路。
    /// 乗り換えた絶対 musical time を返す（サーバーの tempo map へ積むのに要る）。
    pub fn retempo_from_next_step(&mut self, bpm: f64) -> Option<AppliedTempo> {
        if !self.clock.is_running() || bpm == self.clock.bpm() {
            return None;
        }
        let step = self.clock.next_step();
        let at_timeline_seconds = self.clock.retempo(step, bpm)?;
        Some(AppliedTempo {
            bpm,
            at_timeline_seconds,
        })
    }

    /// 次にコード進行を1周したところで乗り換えるテンポを預ける。
    /// 進行の頭が来る前なら何度でも上書きできる。
    pub fn arm_next_cycle_bpm(&mut self, bpm: f64) {
        self.next_cycle_bpm = Some(bpm);
    }

    /// 予約済みか。フレームごとに引き直して捨てるのを避けるために見る。
    pub fn has_armed_cycle_bpm(&self) -> bool {
        self.next_cycle_bpm.is_some()
    }

    /// 予約を取り下げる。テンポを別経路で作り直したときに、古い抽選を残さないために呼ぶ。
    pub fn disarm_next_cycle_bpm(&mut self) {
        self.next_cycle_bpm = None;
        self.applied_cycle_bpm = None;
    }

    /// 進行の頭で乗り換えたテンポを一度だけ報告する。画面側は表示を追従させ、
    /// サーバーの tempo map へ変化点を積む。
    pub fn take_applied_cycle_bpm(&mut self) -> Option<AppliedTempo> {
        self.applied_cycle_bpm.take()
    }
}

#[cfg(test)]
mod tests;
