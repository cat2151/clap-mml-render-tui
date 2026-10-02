use std::time::Instant;

use super::periodic_timeline::{TimelineSend, LOOKAHEAD};
use super::sender::next_timeline_id;
use super::KeyboardScreen;

impl KeyboardScreen<'_> {
    /// 周期送信・patch切替後の再送・音出し確認ガイドを1フレーム分進める。
    ///
    /// 日次 overlay を新たに表示した場合だけ true を返す（保存は共有ランタイム側の責務）。
    pub fn pump_periodic(&mut self, now: Instant, local_date: &str) -> bool {
        let ready =
            self.connection_status().phase.accepts_notes() && !self.periodic_timeline.is_stopped();
        let first_overlay_today = self.note_guide.tick(now, ready, local_date);
        if !ready {
            // 接続し直し・音色の差し替えの間に、張っていた timeline は当てにできなくなる。
            self.periodic_timeline.forget();
            return first_overlay_today;
        }
        // patch切替後の現在値再送(refresh)は即時、周期送信は LOOKAHEAD 先までを予約する
        let refresh = self.state.take_pending_refresh_messages(now);
        let tick = self.state.poll_periodic_tick(now + LOOKAHEAD);
        if self.midi_sender.is_some() {
            let sends = self
                .periodic_timeline
                .plan(now, refresh, tick, next_timeline_id);
            self.dispatch(sends);
        }
        first_overlay_today
    }

    /// 予約済みの周期送信を捨ててから `messages` を即時に送る。
    ///
    /// 呼び出し側は Ready を確かめてから呼ぶこと。
    pub(crate) fn send_after_cancel(&mut self, messages: Vec<[u8; 3]>) {
        if self.midi_sender.is_none() {
            return;
        }
        self.periodic_timeline.request_cancel();
        let sends = self
            .periodic_timeline
            .plan(Instant::now(), messages, None, next_timeline_id);
        self.dispatch(sends);
    }

    /// 画面を出るときの消音。予約を捨ててから `note_offs` を送り、instance を止める。
    ///
    /// 以後、[`Self::start`] か [`Self::resume`] までは周期送信を出さない。
    pub(crate) fn stop_sending(&mut self, note_offs: Vec<[u8; 3]>) {
        let sends = self
            .periodic_timeline
            .plan_stop(note_offs, next_timeline_id);
        self.dispatch(sends);
        if let Some(sender) = &self.midi_sender {
            sender.stop();
        }
    }

    /// 画面を出て周期送信を止めている間か（[`Self::stop_sending`] の後、まだ入り直していない）。
    #[cfg(any(test, feature = "test-support"))]
    pub fn periodic_sending_stopped(&self) -> bool {
        self.periodic_timeline.is_stopped()
    }

    fn dispatch(&self, sends: Vec<TimelineSend>) {
        let Some(sender) = &self.midi_sender else {
            return;
        };
        for send in sends {
            match send {
                TimelineSend::Begin(timeline_id) => sender.begin_timeline(timeline_id),
                TimelineSend::Immediate(messages) => sender.send(messages, self.state.patch()),
                TimelineSend::Scheduled(events) => sender.send_timeline(events),
            }
        }
    }
}
