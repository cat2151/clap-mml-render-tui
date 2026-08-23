//! オーバーレイの音を出す、唯一の入口。
//!
//! **指針: 次の音を鳴らすときは、それまでの演奏を必ず止める。鳴らさないときは
//! 止めない。** 音を出すメソッドはどれも先頭で [`Voice::stop`] を通る。ここを通らずに
//! 音を出す経路を足さないこと。足した瞬間に「誰も note off を出さない」経路が復活する。
//!
//! 止め方は 2 通りあり、鳴っているものの素性で決まる。
//!
//! * 打鍵の生 MIDI だけ → 記録どおりの note off。release が付くのでクリックしない。
//! * live timeline が絡む、または記録がずれている疑いがある → `stop_all`。
//!   サーバー側は `StopAll` → `reset_all(renderers)` → CLAP `processor.reset()` まで
//!   通るので、こちらが何を鳴らしたか覚えていなくても確実に黙る。
//!
//! どちらの経路でも**コマンドは必ず 1 つ以上飛ぶ**。「鳴っていないはずだから何もしない」
//! で早期 return してよいのは [`Sounding`] が「鳴っていない」と言うときだけで、その記録は
//! 送信と同じ場所で更新している。以前はこの判断材料を 2 か所（state 側の `sounding` と
//! line playback 側の `active`）が別々に持ち、互いに相手が止めると思い込んでいた。

use std::time::{Duration, Instant};

use cmrt_chord::TimedMidiEvent;

use super::line_playback::{LineOutcome, LinePlayback};
use super::sink::SoundSink;
use super::sounding::Sounding;
use super::{log_error, log_line};

pub(super) struct Voice {
    line: LinePlayback,
    sounding: Sounding,
    current_patch: Option<String>,
    patch_ready: bool,
    /// worker がいま処理している command。関連ログを他 thread の行と区別するために使う。
    command_id: u64,
    /// note on が server に受理されてから、MML が指定した音長だけ先の停止時刻。
    gate_deadline: Option<Instant>,
}

impl Voice {
    pub(super) fn new(sample_rate_hz: f64) -> Self {
        Self {
            line: LinePlayback::new(sample_rate_hz),
            sounding: Sounding::default(),
            current_patch: None,
            patch_ready: false,
            command_id: 0,
            gate_deadline: None,
        }
    }

    pub(super) fn begin_command(&mut self, command_id: u64) {
        self.command_id = command_id;
    }

    pub(super) fn is_patch_ready(&self, patch: Option<&str>) -> bool {
        self.patch_ready && self.current_patch.as_deref() == patch
    }

    /// 音源をこの音色で使えるようにする。
    ///
    /// 音色の差し替えは音を出さないが、鳴っている最中に差し替えると前の音色の音が
    /// 残る。ここも「鳴らす前」と同じ扱いで止めてから通す。
    pub(super) fn prepare(&mut self, sink: &impl SoundSink, patch: Option<&str>) -> bool {
        self.stop(sink, "prepare");
        log_line(format!(
            "action=mml-overlay-prepare event=start command_id={} patch={patch:?}",
            self.command_id
        ));
        let started_at = Instant::now();
        match sink.prepare_patch(patch) {
            Ok(()) => {
                self.current_patch = patch.map(str::to_string);
                self.patch_ready = true;
                log_line(format!(
                    "action=mml-overlay-prepare event=success command_id={} patch={patch:?} \
                     elapsed_ms={}",
                    self.command_id,
                    started_at.elapsed().as_millis()
                ));
                true
            }
            Err(error) => {
                log_error(format!(
                    "action=mml-overlay-prepare event=error command_id={} patch={patch:?} \
                     elapsed_ms={} active_patch={:?} error=\"{error}\"",
                    self.command_id,
                    started_at.elapsed().as_millis(),
                    self.current_patch
                ));
                false
            }
        }
    }

    /// 打鍵の 1 音を生 MIDI で鳴らす。空で呼ぶと止めるだけになる。
    /// gate は送信成功後から数える。patch load や接続待ちを音長から差し引かない。
    pub(super) fn play_notes(
        &mut self,
        sink: &impl SoundSink,
        messages: &[[u8; 3]],
        gate: Duration,
    ) -> bool {
        self.stop(sink, "notes");
        if messages.is_empty() {
            return false;
        }
        // 記録は送信の成否に関わらず付ける。届いていたのに記録が無いと、次の停止で
        // note off が出ずに鳴りっぱなしになる（届かなかった側へ余計な note off が
        // 出るのは無害）。
        self.sounding.record_sent(messages);
        match sink.send_midi(messages) {
            Ok(()) => {
                let submitted_at = Instant::now();
                self.sounding.mark_note_submitted(submitted_at);
                self.gate_deadline = Some(submitted_at + gate);
                log_line(format!(
                    "action=mml-overlay-send event=success command_id={} patch={:?} pitches={:?} \
                     gate_ms={}",
                    self.command_id,
                    self.current_patch,
                    note_on_pitches(messages),
                    gate.as_millis()
                ));
                true
            }
            Err(error) => {
                self.sounding
                    .mark_suspect(&format!("send-midi-failed error=\"{error}\""));
                log_error(format!(
                    "action=mml-overlay-send event=error command_id={} patch={:?} pitches={:?} \
                     error=\"{error}\"",
                    self.command_id,
                    self.current_patch,
                    note_on_pitches(messages)
                ));
                false
            }
        }
    }

    pub(super) fn gate_wait(&self, now: Instant) -> Option<Duration> {
        self.gate_deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    /// 1 行ぶんのフレーズを live timeline へ積む。空で呼ぶと止めるだけになる。
    ///
    /// **ここで送る note off は、サーバーに届いても適用されないことがある。**
    /// サーバーのコマンドキューは `submit_begin_live_timeline` で `pending.clear()` を
    /// するので、note off を積んだ直後に timeline を張ると worker が拾う前に捨てられる
    /// （実測ログで `kind=midi [i0:80:60:0,..]` は受け口に届いていたのに `apply-midi` が
    /// 出ていない）。最後に音を止めるのはサーバー側の renderer リセットで、そちらは
    /// All Sound Off を流すようにしてある。ここが note off を送るのはそれでも意味が
    /// あって、届いたときは release 付きで musical に切れる。
    pub(super) fn play_line(&mut self, sink: &impl SoundSink, events: &[TimedMidiEvent]) {
        self.stop(sink, "line");
        // 空行やエラー行はここで終わる。以前はこの経路でサーバーへ何も飛ばず、
        // 直前に打鍵した音が鳴り続けていた。いまは上の stop で必ず止まっている。
        if events.is_empty() {
            return;
        }
        match self.line.play(sink, events) {
            LineOutcome::Playing => self.sounding.begin_timeline(),
            LineOutcome::Partial => {
                // 積み終えた note on の note off だけ落ちた可能性がある。
                self.sounding.begin_timeline();
                self.sounding.mark_suspect("line-send-incomplete");
            }
            // timeline を張れていないので音は出ていない。記録は増やさない。
            LineOutcome::Failed => self.sounding.mark_suspect("line-begin-failed"),
        }
    }

    /// 鳴っているものを全部止める。
    ///
    /// `reason` は log 用。どの操作が止めに来たのかが分からないと、鳴りっぱなしを
    /// 追うときに「そもそも止めに来ていない」のか「止めたのに鳴っている」のかを
    /// 切り分けられない。
    pub(super) fn stop(&mut self, sink: &impl SoundSink, reason: &str) {
        self.gate_deadline = None;
        if self.sounding.is_silent() && !self.sounding.needs_hard_stop() {
            return;
        }
        let mut hard = self.sounding.needs_hard_stop();
        let note_offs = self.sounding.note_offs();
        // ここで測るのは音声波形ではなく、note on の送信成功から停止処理開始までの窓。
        // stop のログ書き込み時間を窓へ混ぜないため、ログより先に時刻を確定する。
        let note_window_ms = self.sounding.note_window_ms(Instant::now());
        log_line(format!(
            "action=mml-overlay-stop event=start command_id={} reason={reason} {} hard={hard} \
             patch={:?} note_window_ms={} audibility={}",
            self.command_id,
            self.sounding.describe(),
            self.current_patch,
            optional_ms(note_window_ms),
            audibility(note_window_ms)
        ));
        if !hard && !note_offs.is_empty() {
            if let Err(error) = sink.send_midi(&note_offs) {
                // note off が届いていない。音源ごと止めるほうへ倒す。
                log_error(format!(
                    "action=mml-overlay-stop event=note-off-error error=\"{error}\""
                ));
                hard = true;
            }
        }
        if hard {
            if let Err(error) = sink.stop_all() {
                log_error(format!(
                    "action=mml-overlay-stop event=error error=\"{error}\""
                ));
                // 音源が黙った保証が無い。疑いを残したまま次の停止へ持ち越す。
                self.sounding.clear(false);
                return;
            }
        }
        self.sounding.clear(hard);
        log_line(format!(
            "action=mml-overlay-stop event=success command_id={} reason={reason} patch={:?} \
             note_window_ms={} audibility={}",
            self.command_id,
            self.current_patch,
            optional_ms(note_window_ms),
            audibility(note_window_ms)
        ));
    }
}

fn note_on_pitches(messages: &[[u8; 3]]) -> Vec<u8> {
    messages
        .iter()
        .filter(|message| message[0] == crate::NOTE_ON && message[2] > 0)
        .map(|message| message[1])
        .collect()
}

/// 音声サンプルそのものは client から観測できないため、発音成否を断定しない。
/// ただし note on と note off が同じ ms に server へ渡ったケースは、今回の無音と一致する。
fn audibility(note_window_ms: Option<u128>) -> &'static str {
    match note_window_ms {
        Some(0) => "at-risk-zero-window",
        // 48 kHz / 512 frames の1 blockが約11ms。20ms以下だと note on/off が
        // 同じ処理時刻へ潰れる可能性が高いので、無音を断定せず risk として残す。
        Some(1..=20) => "at-risk-short-window",
        Some(_) => "unverified-nonzero-window",
        None => "unverified-no-note-window",
    }
}

fn optional_ms(value: Option<u128>) -> String {
    value.map_or_else(|| "unknown".to_string(), |value| value.to_string())
}

#[cfg(test)]
mod tests;
