use std::time::{Duration, Instant};

use cmrt_realtime_play::{fast_midi_ipc::MAX_MIDI_MESSAGES, TimelineMidiEvent};

use super::{GridScheduledMessage, GridSequencerContext, GridSequencerScreen};

/// コード進行データ更新のアナウンスを出しておく時間。読めるだけの長さがあればよい。
const RESTART_NOTICE_DURATION: Duration = Duration::from_secs(3);

impl GridSequencerScreen {
    pub(crate) fn restart_timeline(&mut self, now: Instant) {
        self.timeline_id = self
            .midi_sender
            .as_ref()
            .map(|sender| sender.begin_timeline(self.sample_rate, self.bpm()))
            .unwrap_or(1);
        self.state.start_at_bpm(now, self.bpm());
    }

    /// 先読み分のステップを組み立て、offset つきでまとめて送る。
    ///
    /// 接続前・音色切替中は進めない。クロックの締切はそのまま残り、Ready 復帰時に
    /// now 基準へ張り直す（欠落ぶんをまとめて鳴らさない）。
    ///
    /// コード進行データの更新アナウンスを出し終えたら true を返す。共有ランタイムは
    /// これを受けてアプリを再起動する。
    pub fn pump_step(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) -> bool {
        let status = self.connection_status();
        // 上限バッファでもフレームドロップが止まらない環境では、裏読みが成立しない。
        if status.overloaded && !self.overload_applied {
            self.enter_single_buffering("overload");
        }
        // 次の周のテンポを先に予約しておく。組み立ては先読みするので、周の頭が
        // 来る前に預けておかないと1周ぶん取り逃がす。
        self.arm_next_cycle_bpm();
        if self.poll_start_wait(now, status.phase.accepts_notes()) {
            let bank_before = self.state.bank();
            let chord_before = self
                .state
                .chord()
                .map(|chord| (chord.index(), chord.chord_count()));
            let scheduled = self
                .state
                .poll_steps(now, self.scheduling_lookahead(status.buffer_multiplier));
            // 周の頭を跨いだならテンポが乗り換わっている。表示を追従させる。
            self.absorb_applied_cycle_bpm();
            self.log_chord_schedule(&scheduled, bank_before, chord_before);
            self.send_scheduled_with_lateness(&scheduled, self.state.last_poll_lateness());
            if self.single_buffering {
                // 鳴らしきってからロードする。この間は演奏が止まる。
                self.advance_single_buffer_cycle(now, ctx);
            } else {
                // 進行の最終小節へ入っていれば、待機 bank へ次サイクルを先読みする。
                // 演奏は止まらない（差し替えは次の小節境界で起きる）。
                self.advance_cycle_swap(now, ctx);
            }
        }
        self.expire_patch_notice(now);
        self.take_restart_request(now)
    }

    /// 表示時間が過ぎた patch selector の通知を消す。操作を塞がないので、
    /// 消えるきっかけは時間だけ（[`crate::patch_notice`]）。
    fn expire_patch_notice(&mut self, now: Instant) {
        if self
            .patch_notice
            .as_ref()
            .is_some_and(|notice| notice.expired(now))
        {
            self.patch_notice = None;
        }
    }

    /// 再起動アナウンスの表示時間が過ぎていたら、一度だけ true を返す。
    fn take_restart_request(&mut self, now: Instant) -> bool {
        let Some(since) = self.restart_notice else {
            return false;
        };
        if now.saturating_duration_since(since) < RESTART_NOTICE_DURATION {
            return false;
        }
        self.restart_notice = None;
        true
    }

    /// 組み立て済みのメッセージを、`ahead` をフレーム数へ直して送る。
    pub(crate) fn send_scheduled(&self, scheduled: &[GridScheduledMessage]) {
        self.send_scheduled_with_lateness(scheduled, Duration::ZERO);
    }

    fn send_scheduled_with_lateness(
        &self,
        scheduled: &[GridScheduledMessage],
        pump_lateness: Duration,
    ) {
        if scheduled.is_empty() {
            return;
        }
        let Some(sender) = &self.midi_sender else {
            return;
        };
        for batch in batches(scheduled, self.timeline_id) {
            sender.send_scheduled(batch, pump_lateness);
        }
    }

    /// Keep enough absolute events queued for the next adaptive-buffer level as well as UI/IPC
    /// scheduling jitter. A larger output lead must never move the musical phase.
    fn scheduling_lookahead(&self, multiplier: u16) -> Duration {
        let next_multiplier = multiplier
            .saturating_mul(2)
            .min(cmrt_realtime_play::MAX_LIVE_BUFFER_MULTIPLIER);
        let two_buffer_leads = Duration::from_secs_f64(
            2.0 * self.buffer_frames as f64 * f64::from(next_multiplier) / self.sample_rate,
        );
        two_buffer_leads + crate::lookahead_at(self.bpm()) + Duration::from_millis(50)
    }

    /// patch selector の調査用に、低頻度な chord attack と bank 遷移だけを記録する。
    /// 全行の Note On を記録すると通常演奏で log が埋まるため、和音行へ限定する。
    fn log_chord_schedule(
        &self,
        scheduled: &[GridScheduledMessage],
        bank_before: usize,
        chord_before: Option<(usize, usize)>,
    ) {
        let bank_after = self.state.bank();
        let chord_after = self
            .state
            .chord()
            .map(|chord| (chord.index(), chord.chord_count()));
        if bank_before != bank_after || chord_before != chord_after {
            crate::log_line(&format!(
                "grid-sequencer: chord-transport random={} chord={chord_before:?}->{chord_after:?} \
                 active_bank={bank_before}->{bank_after}",
                self.cycle_random.compact_label(),
            ));
        }

        let instance_count = self.state.instance_count();
        let attacks = scheduled
            .iter()
            .filter(|event| {
                event.message[0] & 0xf0 == 0x90
                    && event.message[2] != 0
                    && usize::from(event.instance_id) % instance_count == crate::CHORD_ROW
            })
            .map(|event| {
                format!(
                    "bank:{} instance:{} note:{} ahead_ms:{}",
                    usize::from(event.instance_id) / instance_count,
                    event.instance_id,
                    event.message[1],
                    event.ahead.as_millis(),
                )
            })
            .collect::<Vec<_>>();
        if attacks.is_empty() {
            return;
        }
        crate::log_line(&format!(
            "grid-sequencer: chord-note-on random={} active_bank={} chord={chord_after:?} \
             logical_patch={:?} events=[{}]",
            self.cycle_random.compact_label(),
            bank_after,
            self.state.instances()[crate::CHORD_ROW].patch.as_deref(),
            attacks.join(", "),
        ));
    }
}

/// 送信単位へ切り分ける。
///
/// 同じ `ahead` のメッセージは必ず1回の送信へまとめる。1バッチの上限は共有メモリの
/// スロット容量。
///
/// 「同じ `ahead` ＝同じステップ」は swing（[`crate::state::swing`]）が入ってからは
/// 成り立たない。同じステップでも跳ねた instance は後ろの group へ回る。発音位置は
/// 絶対 `timeline_seconds` で決まり、サーバーは整列挿入する（play-server の
/// `BlockScheduler::schedule`）ので、ステップが複数バッチへ割れても位置は変わらない。
fn batches(scheduled: &[GridScheduledMessage], timeline_id: u64) -> Vec<Vec<TimelineMidiEvent>> {
    let mut batches: Vec<Vec<TimelineMidiEvent>> = Vec::new();
    let mut current: Vec<TimelineMidiEvent> = Vec::new();
    for group in group_by_ahead(scheduled) {
        if !current.is_empty() && current.len() + group.len() > MAX_MIDI_MESSAGES {
            batches.push(std::mem::take(&mut current));
        }
        current.extend(group.iter().map(|scheduled| TimelineMidiEvent {
            timeline_id,
            instance_id: scheduled.instance_id,
            timeline_seconds: scheduled.timeline_seconds,
            message: scheduled.message,
        }));
    }
    if !current.is_empty() {
        batches.push(current);
    }
    batches
}

/// 同じ `ahead` を持つ連続したメッセージへ切り分ける。`poll_steps` がステップ内を
/// 時刻順へ並べてくるので、跳ねた instance と跳ねない instance が交互になることはない。
fn group_by_ahead(scheduled: &[GridScheduledMessage]) -> Vec<&[GridScheduledMessage]> {
    let mut groups = Vec::new();
    let mut start = 0;
    for index in 1..=scheduled.len() {
        let ended = index == scheduled.len() || scheduled[index].ahead != scheduled[start].ahead;
        if ended {
            groups.push(&scheduled[start..index]);
            start = index;
        }
    }
    groups
}

#[cfg(test)]
mod tests;
