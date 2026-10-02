//! 周期送信（arp・repeat・周期 CC/PB）を live timeline 経路で送るための組み立て。
//!
//! tick はその deadline の絶対秒で予約し、deadline より [`LOOKAHEAD`] だけ前に送る。
//! TUI の frame が遅れても、遅れが `LOOKAHEAD` 未満なら音の時刻は動かない。
//!
//! 予約済みの音を取り消す手段は「新しい id の timeline を張る」ことしか無い
//! （play server の `StopInstance` は予約を消さない）。張り直すとサーバーは
//! 鳴っている音をすべて離すので、手で押さえている音も止まる。

use std::time::{Duration, Instant};

use cmrt_realtime_play::{TimelineId, TimelineMidiEvent};

use crate::PeriodicTick;

/// tick を deadline よりどれだけ前に送るか。
///
/// 250ms（tick の間隔）未満に保つこと。それ以上だと 1 回の poll で tick が 2 つ出る
/// べき場面があり、`poll_periodic_tick` は高々 1 つしか返さない。
pub(crate) const LOOKAHEAD: Duration = Duration::from_millis(200);

/// 送る順に並んだ 1 回分の送信。
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TimelineSend {
    /// この id で timeline を張り直す。予約済みの音は捨てられる。
    Begin(TimelineId),
    /// 受け取った時点で鳴らす。
    Immediate(Vec<[u8; 3]>),
    /// timeline 上の絶対秒で予約する。
    Scheduled(Vec<TimelineMidiEvent>),
}

#[derive(Clone, Copy, Debug)]
struct ActiveTimeline {
    id: TimelineId,
    /// timeline の 0 秒に当たる時刻（Begin を送った時刻）。
    origin: Instant,
}

/// keyboard 画面が張っている timeline と、取り消し要求。
#[derive(Debug, Default)]
pub(crate) struct PeriodicTimeline {
    active: Option<ActiveTimeline>,
    cancel_requested: bool,
    /// [`Self::plan_stop`] から [`Self::restart`] までの間。周期送信を出さない。
    stopped: bool,
}

impl PeriodicTimeline {
    /// 画面へ入る・戻るときに呼ぶ。止めていた周期送信を許し、次の送信で張り直す。
    pub(crate) fn restart(&mut self) {
        *self = Self::default();
    }

    /// 画面を出た（音源を明け渡した）後で、まだ [`Self::restart`] されていないか。
    ///
    /// 止めた直後は worker が Stop を処理し終えるまで接続が Ready のまま見えるので、
    /// 接続状態だけでは「もう送ってはいけない」を判定できない。
    pub(crate) fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// 次の送信の前に timeline を張り直し、予約済みの音を捨てる。
    pub(crate) fn request_cancel(&mut self) {
        self.cancel_requested = true;
    }

    /// 張っていた timeline が他者に差し替えられた・サーバーとの接続が切れた等で、
    /// もう当てにできないときに忘れる。次の送信で張り直す。
    ///
    /// 止めている状態（[`Self::is_stopped`]）は解かない。
    pub(crate) fn forget(&mut self) {
        self.active = None;
        self.cancel_requested = false;
    }

    /// Ready の間の 1 回の送信を組み立てる。順序は Begin → 即時 → 予約。
    ///
    /// timeline を張っていないか取り消し要求があれば、`next_id` の id で張り直す。
    pub(crate) fn plan(
        &mut self,
        now: Instant,
        immediate: Vec<[u8; 3]>,
        tick: Option<PeriodicTick>,
        next_id: impl FnOnce() -> TimelineId,
    ) -> Vec<TimelineSend> {
        let mut sends = Vec::new();
        let active = match self.active {
            Some(active) if !self.cancel_requested => active,
            _ => {
                let active = ActiveTimeline {
                    id: next_id(),
                    origin: now,
                };
                sends.push(TimelineSend::Begin(active.id));
                self.active = Some(active);
                self.cancel_requested = false;
                active
            }
        };
        if !immediate.is_empty() {
            sends.push(TimelineSend::Immediate(immediate));
        }
        if let Some(tick) = tick.filter(|tick| !tick.messages.is_empty()) {
            sends.push(TimelineSend::Scheduled(scheduled_events(active, &tick)));
        }
        sends
    }

    /// 画面を出るときの送信。張っていれば空の新しい timeline で予約を捨ててから
    /// `immediate` を送り、以後は [`Self::restart`] まで止めておく。
    pub(crate) fn plan_stop(
        &mut self,
        immediate: Vec<[u8; 3]>,
        next_id: impl FnOnce() -> TimelineId,
    ) -> Vec<TimelineSend> {
        let mut sends = Vec::new();
        if self.active.is_some() {
            sends.push(TimelineSend::Begin(next_id()));
        }
        self.forget();
        self.stopped = true;
        if !immediate.is_empty() {
            sends.push(TimelineSend::Immediate(immediate));
        }
        sends
    }
}

fn scheduled_events(active: ActiveTimeline, tick: &PeriodicTick) -> Vec<TimelineMidiEvent> {
    let timeline_seconds = tick
        .at
        .saturating_duration_since(active.origin)
        .as_secs_f64();
    tick.messages
        .iter()
        .map(|&message| TimelineMidiEvent {
            timeline_id: active.id,
            instance_id: crate::sender::KEYBOARD_INSTANCE,
            timeline_seconds,
            message,
        })
        .collect()
}

#[cfg(test)]
mod tests;
