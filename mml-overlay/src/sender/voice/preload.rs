//! 鳴っていない bank へ、次に使う音色を裏で読んでおく。
//!
//! 受付と完了の確認を分け、**worker は読み込みの完了を待たない**。待つとその間
//! 次の command を処理できず、鳴らしたい行が読み込みの長さだけ遅れる。完了は worker が
//! 空いた時間に [`Voice::poll_preload`] で見る。
//!
//! 先読みは取り消せず、同時に 1 件しか持てない。読み込み中に別の音色を読む必要が
//! 出た command は、[`Voice::settle_preload`] で先読みの決着を待ってから読む。

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use super::super::live_patch::LivePatch;
use super::super::sink::{PreloadTicket, SoundSink};
use super::super::status::MmlOverlayPreload;
use super::super::{log_error, log_line};
use super::{PatchState, Voice};

const SHUTTING_DOWN: &str = "sender is shutting down";

/// 読み込み中の先読みの完了を見に行く間隔。
pub(in crate::sender) const PRELOAD_POLL_INTERVAL: Duration = Duration::from_millis(5);

pub(in crate::sender) struct InFlightPreload {
    ticket: PreloadTicket,
    instance_id: u8,
    patch: LivePatch,
    started_at: Instant,
}

impl Voice {
    /// `patch` を鳴っていない bank の instance へ読み始める。完了は待たない。
    ///
    /// どちらかの instance で既に使えるなら読まない。別の先読みが読み込み中なら、それが
    /// 決着してから読む（後から頼んだ 1 件だけを覚える）。
    pub(in crate::sender) fn request_preload(&mut self, sink: &impl SoundSink, patch: LivePatch) {
        if let Some(instance_id) = self.ready_instance_of(sink, &patch) {
            self.preloaded = Some((instance_id, patch));
            return;
        }
        match &self.preload {
            Some(in_flight) if in_flight.patch == patch => {}
            Some(_) => {
                log_line(format!(
                    "action=mml-overlay-preload event=queued {}",
                    patch.log_fields()
                ));
                self.queued_preload = Some(patch);
            }
            None => self.begin_preload(sink, patch),
        }
    }

    /// 読み込み中の先読みの完了を 1 回だけ見る。block しない。
    pub(in crate::sender) fn poll_preload(&mut self, sink: &impl SoundSink) {
        let Some(in_flight) = self.preload.as_mut() else {
            return;
        };
        let result = match sink.poll_preload(&mut in_flight.ticket) {
            Ok(false) => return,
            Ok(true) => Ok(()),
            Err(error) => Err(error),
        };
        let in_flight = self.preload.take().expect("the preload was just polled");
        let fields = in_flight.patch.log_fields();
        let elapsed_ms = in_flight.started_at.elapsed().as_millis();
        match result {
            Ok(()) => {
                log_line(format!(
                    "action=mml-overlay-preload event=success instance={} {fields} \
                     elapsed_ms={elapsed_ms}",
                    in_flight.instance_id
                ));
                self.patches.insert(
                    in_flight.instance_id,
                    PatchState {
                        current: in_flight.patch.clone(),
                        ready: true,
                    },
                );
                self.preloaded = Some((in_flight.instance_id, in_flight.patch));
            }
            Err(error) => log_error(format!(
                "action=mml-overlay-preload event=error instance={} {fields} \
                 elapsed_ms={elapsed_ms} error=\"{error}\"",
                in_flight.instance_id
            )),
        }
        if let Some(next) = self.queued_preload.take() {
            self.request_preload(sink, next);
        }
    }

    /// 先読みが決着するまで待つ。音色を読む前に呼ぶ（同時に 2 件は読めない）。
    ///
    /// sender が畳まれ始めたら待たずに先読みを手放し、`Err` を返す。先読みは数十秒かかることがあり、
    /// その間 `Drop` の join がアプリの終了を塞ぐ。server は先読みが決着するまで次の音色を読まないので、
    /// `Err` を受けた側は音色を読みに行かない。
    pub(in crate::sender) fn settle_preload(
        &mut self,
        sink: &impl SoundSink,
    ) -> Result<(), String> {
        while self.preload.is_some() {
            if self.shutting_down.load(Ordering::Acquire) {
                self.abandon_preload(sink);
                return Err(SHUTTING_DOWN.to_string());
            }
            self.poll_preload(sink);
            if self.preload.is_some() {
                std::thread::sleep(PRELOAD_POLL_INTERVAL);
            }
        }
        Ok(())
    }

    /// 先読みを手放す。sender を畳むときに使う。
    pub(in crate::sender) fn abandon_preload(&mut self, sink: &impl SoundSink) {
        self.queued_preload = None;
        if let Some(in_flight) = self.preload.take() {
            log_line(format!(
                "action=mml-overlay-preload event=abandoned instance={} {}",
                in_flight.instance_id,
                in_flight.patch.log_fields()
            ));
            sink.abandon_preload(in_flight.ticket);
        }
    }

    pub(in crate::sender) fn preload_wait(&self) -> Option<Duration> {
        self.preload.as_ref().map(|_| PRELOAD_POLL_INTERVAL)
    }

    /// 画面へ見せる先読みの状態。読み終えた bank を別の音色が上書きしたら `None`。
    pub(in crate::sender) fn preload_state(&self) -> Option<MmlOverlayPreload> {
        if let Some(in_flight) = &self.preload {
            return Some(MmlOverlayPreload::Loading(in_flight.patch.clone()));
        }
        self.preloaded
            .as_ref()
            .filter(|(instance_id, patch)| self.is_patch_ready(*instance_id, patch))
            .map(|(_, patch)| MmlOverlayPreload::Ready(patch.clone()))
    }

    fn ready_instance_of(&self, sink: &impl SoundSink, patch: &LivePatch) -> Option<u8> {
        let current = self.line_instance;
        let standby = sink.standby_instance_of(current);
        [Some(current), standby]
            .into_iter()
            .flatten()
            .find(|&instance_id| self.is_patch_ready(instance_id, patch))
    }

    fn begin_preload(&mut self, sink: &impl SoundSink, patch: LivePatch) {
        let fields = patch.log_fields();
        let Some(instance_id) = sink.standby_instance_of(self.line_instance) else {
            log_error(format!(
                "action=mml-overlay-preload event=error {fields} error=\"no standby bank\""
            ));
            return;
        };
        let started_at = Instant::now();
        // 読み終えるまで中身は分からない。成功するまで準備済みにしない。
        self.patches.remove(&instance_id);
        match sink.begin_preload(instance_id, &patch) {
            Ok(ticket) => {
                log_line(format!(
                    "action=mml-overlay-preload event=accepted instance={instance_id} {fields} \
                     elapsed_ms={}",
                    started_at.elapsed().as_millis()
                ));
                self.preload = Some(InFlightPreload {
                    ticket,
                    instance_id,
                    patch,
                    started_at,
                });
            }
            Err(error) => log_error(format!(
                "action=mml-overlay-preload event=error instance={instance_id} {fields} \
                 elapsed_ms={} error=\"{error}\"",
                started_at.elapsed().as_millis()
            )),
        }
    }
}
