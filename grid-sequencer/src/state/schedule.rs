//! 締切が来たステップを組み立て、送出用の [`GridScheduledMessage`] にする。
//!
//! クロック（[`super::clock`]）が「いつ」を、[`super::attack`] と [`super::cc1`] が
//! 「何を」を決める。ここはその2つを繋いで、先読み・swing・表示位置の遅延を乗せる。

use std::time::{Duration, Instant};

use super::{
    note_off, presentation::PendingDisplay, swing_offset_seconds, GridScheduledMessage, GridState,
    GRID_STEPS,
};

impl GridState {
    /// `now + lookahead` までに鳴るステップをまとめて組み立て、送るべき MIDI メッセージを
    /// 「今から鳴るまでの時間」つきで返す。締切がまだ先なら空を返す。
    ///
    /// 先読みして送るので、UI のポーリング間隔ぶんのジッタが発音位置に乗らない。
    pub fn poll_steps(&mut self, now: Instant, lookahead: Duration) -> Vec<GridScheduledMessage> {
        let mut scheduled = Vec::new();
        self.last_poll_lateness = Duration::ZERO;
        for due in self.clock.take_due(now, lookahead) {
            let deadline = due.deadline;
            self.last_poll_lateness = self
                .last_poll_lateness
                .max(now.saturating_duration_since(deadline));
            let ahead = deadline.saturating_duration_since(now);
            let timeline_seconds = self.clock.timeline_seconds(due.step);
            let mut messages = self.expire_sounding();
            self.advance_schedule(due.step);
            let stopping = std::mem::take(&mut self.cycle_wrapped);
            if stopping {
                // 鳴らしきった。次の小節は組み立てず、残っている音を止めてクロックを畳む。
                messages.extend(self.silence_sounding());
                self.clock.stop();
                self.cycle_stopped_at = Some(deadline);
            } else {
                if self.schedule_index == 0 {
                    self.prepare_lane_measures(deadline);
                }
                // CC1 は発音中にも効くので、note on の有無に関わらず全stepで送る。
                // note on より前に置くのは、鳴り始めの値を確実に載せるため。
                messages.extend(self.cc1_messages_for_step());
                messages.extend(self.attack_current_step());
            }
            scheduled.extend(self.apply_swing(messages, ahead, timeline_seconds, stopping));
            self.pending_display.push_back(PendingDisplay {
                deadline,
                step: self.schedule_index,
                presentation: self.capture_presentation(),
            });
            self.last_scheduled = Some(deadline);
            self.last_scheduled_timeline_seconds = Some(timeline_seconds);
            if stopping {
                break;
            }
        }
        self.advance_display(now);
        scheduled
    }

    /// 組み立て済みの1ステップぶんへ、instance ごとの swing を乗せる（[`super::swing`]）。
    ///
    /// note off（`expire_sounding`）・CC1・note on はどれも `(instance_id, message)` なので、
    /// ここ1箇所でまとめてずれる。note on だけ遅らせて note off を置き去りにする事故が
    /// 構造的に起きない。
    ///
    /// `stopping`（鳴らしきりの note off）は跳ねさせない。止めるだけの音を遅らせる理由がない。
    fn apply_swing(
        &self,
        messages: Vec<(u8, [u8; 3])>,
        ahead: Duration,
        timeline_seconds: f64,
        stopping: bool,
    ) -> Vec<GridScheduledMessage> {
        let swings = if stopping {
            Vec::new()
        } else {
            self.effective_swings()
        };
        let instance_count = self.instance_count().max(1);
        let step = self.schedule_index;
        let bpm = self.clock.bpm();
        let mut step_messages = messages
            .into_iter()
            .map(|(instance_id, message)| {
                let offset = swings
                    .get(usize::from(instance_id) % instance_count)
                    .copied()
                    .flatten()
                    .map_or(0.0, |swing| swing_offset_seconds(swing, step, bpm));
                GridScheduledMessage {
                    instance_id,
                    // サーバーが並べているのは `timeline_seconds` のほう。`ahead` だけ
                    // ずらしても実機では効かないので、必ず両方へ乗せる。
                    ahead: ahead + Duration::from_secs_f64(offset),
                    timeline_seconds: timeline_seconds + offset,
                    message,
                }
            })
            .collect::<Vec<_>>();
        // 送出順を時刻順へ揃える。跳ねた instance が跳ねない instance より先に並ぶと、
        // 送信ストリームの `timeline_seconds` が step の内側で巻き戻る。
        // **stable sort** であることが要る。同時刻の「note off → CC1 → note on」と
        // 行順は、組み立てた順のままでなければならない。
        step_messages
            .sort_by(|left, right| left.timeline_seconds.total_cmp(&right.timeline_seconds));
        step_messages
    }

    pub(crate) fn last_poll_lateness(&self) -> Duration {
        self.last_poll_lateness
    }

    /// 鳴っている音を止める note off を、送信済みの先読みぶんより後ろへ置くための猶予。
    /// まだ何も送っていなければ即座（0）で良い。
    ///
    /// 半ステップの猶予は swing の最大ずれ（0.32 ステップ）より広い。跳ねた note on を
    /// 先回りして止めてしまわないのはこのため。
    pub(crate) fn silence_ahead(&self, now: Instant) -> Duration {
        match self.last_scheduled {
            Some(deadline) => {
                (deadline + self.clock.schedule_guard()).saturating_duration_since(now)
            }
            None => Duration::ZERO,
        }
    }

    pub(crate) fn silence_timeline_seconds(&self) -> f64 {
        self.last_scheduled_timeline_seconds
            .map(|seconds| seconds + self.clock.schedule_guard().as_secs_f64())
            .unwrap_or(0.0)
    }

    /// 鳴っている音をすべて止める note off を作り、再生位置とクロックをリセットする。
    /// 画面を離れるときに呼び、音が鳴りっぱなしになるのを防ぐ。
    pub fn take_reset_messages(&mut self) -> Vec<GridScheduledMessage> {
        self.clock.stop();
        self.step_index = 0;
        self.schedule_index = 0;
        self.started = false;
        self.reset_lanes_for_start();
        self.pending_display.clear();
        self.display = None;
        self.last_scheduled = None;
        self.last_scheduled_timeline_seconds = None;
        self.reset_cycle_stop();
        self.silence_sounding()
            .into_iter()
            .map(|(instance_id, message)| GridScheduledMessage {
                instance_id,
                ahead: Duration::ZERO,
                timeline_seconds: 0.0,
                message,
            })
            .collect()
    }

    /// 締切を過ぎたステップまで表示位置を進める。先読みぶんが先走って見えるのを防ぐ。
    fn advance_display(&mut self, now: Instant) {
        while self
            .pending_display
            .front()
            .is_some_and(|pending| pending.deadline <= now)
        {
            let pending = self
                .pending_display
                .pop_front()
                .expect("front was just observed");
            self.step_index = pending.step;
            self.display = Some(pending.presentation);
        }
        self.advance_lane_displays(now);
    }

    /// 鳴っている音の残りステップを1減らし、尽きたものの note off を返す。
    fn expire_sounding(&mut self) -> Vec<(u8, [u8; 3])> {
        let mut messages = Vec::new();
        self.sounding.retain_mut(|note| {
            note.remaining_steps = note.remaining_steps.saturating_sub(1);
            if note.remaining_steps == 0 {
                messages.push((note.instance_id, note_off(note.midi_note)));
                false
            } else {
                true
            }
        });
        messages
    }

    /// `step` は、いま組み立てているステップの通し番号。周の頭でテンポを乗り換えるのに要る。
    fn advance_schedule(&mut self, step: u64) {
        if self.started {
            self.schedule_index = (self.schedule_index + 1) % GRID_STEPS;
            if self.schedule_index == 0 {
                // grid を1周したので、chord mode なら次のコードへ進む。
                let progression_wrapped = self.advance_chord();
                // テンポの引き直しはコード進行1周ごと。小節（grid 1周）ごとに変えると
                // 進行の途中でテンポが動き、フレーズが繋がらない。chord mode を使って
                // いない間は進行という単位が無いので、従来どおり grid 1周を単位にする。
                if progression_wrapped || self.chord.is_none() {
                    self.apply_next_cycle_bpm(step);
                }
            }
        } else {
            self.started = true;
        }
    }

    /// 鳴っている音の note off だけを作り、発音中リストを空にする。
    pub(super) fn silence_sounding(&mut self) -> Vec<(u8, [u8; 3])> {
        self.sounding
            .drain(..)
            .map(|note| (note.instance_id, note_off(note.midi_note)))
            .collect()
    }
}
