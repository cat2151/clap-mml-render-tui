//! Guitar Articulation 画面と TuiApp を接続する glue。
//!
//! 画面ロジック（状態・キー処理・描画・変換）は `cmrt-guitar-articulation` crate に閉じている。
//! 音を鳴らす手段（`MmlOverlaySender`）を持つのは app 側なので、画面が返した
//! 「この版を鳴らしてほしい」をここで `play_line` へ渡す。

use crossterm::event::KeyEvent;

use cmrt_mml_overlay::line_play::{LinePerformance, LineProgram};
use cmrt_mml_overlay::LivePatch;

use crate::guitar_articulation_events::play_log_line;
use crate::tui::guitar_articulation::{GuitarArticulationAction, Take, PATCH};
use crate::tui::TuiApp;

impl TuiApp<'_> {
    /// キー 1 つを画面へ渡し、演奏の要求があればその場で鳴らす。
    /// [`GuitarArticulationAction::Quit`]（`q`）はランタイムがループを抜けるので、そのまま返す。
    pub(in crate::tui) fn handle_guitar_articulation_key_event(
        &mut self,
        key: KeyEvent,
    ) -> GuitarArticulationAction {
        let action = self.guitar_articulation.handle_key_event(key);
        match action {
            GuitarArticulationAction::Play(take) => self.play_guitar_articulation(take),
            GuitarArticulationAction::Quit => self.stop_guitar_articulation(),
            GuitarArticulationAction::Continue => {}
        }
        action
    }

    /// その版のイベント列を、MIDI filter を通さず 1 回だけ鳴らす。
    /// 前の演奏を止めるのは sender の責務（`play_line` は鳴っているものを止めてから積む）。
    fn play_guitar_articulation(&mut self, take: Take) {
        let events = self.guitar_articulation.events(take).to_vec();
        crate::logging::global_log_sink(&play_log_line(
            take,
            &events,
            self.guitar_articulation.mml(),
            self.guitar_articulation.rules(),
        ));
        // play server が上がっていなければ sender が無い。落とさず、何もしない。
        if let Some(sender) = &self.mml_overlay_sender {
            let loop_seconds = events.last().map_or(0.0, |event| event.seconds);
            sender.play_line(
                LivePatch::new(Some(PATCH)),
                LineProgram::once(LinePerformance {
                    events,
                    loop_seconds,
                }),
            );
        }
    }

    /// 画面に入った時点で METAL-GTX を読み始める。読み込みは数秒かかるので、待ちは入った瞬間の
    /// 中央 overlay（`sound_startup_overlay`）で見せる。読み込み済みなら sender は何も読まず、overlay も出ない。
    /// MML が空なら既定の MML を鳴らす（sender が読み込みを待ってから鳴らす）。
    pub(in crate::tui) fn enter_guitar_articulation(&mut self) {
        crate::logging::global_log_sink(&format!(
            "guitar-articulation: event=prepare patch={PATCH:?}"
        ));
        if let Some(sender) = &self.mml_overlay_sender {
            sender.prepare(LivePatch::new(Some(PATCH)));
        }
        if let GuitarArticulationAction::Play(take) = self.guitar_articulation.enter() {
            self.play_guitar_articulation(take);
        }
    }

    /// 前回この画面で終えて起動したとき、メニューから入ったときと同じ準備をする。
    pub(in crate::tui) fn enter_restored_guitar_articulation(&mut self) {
        if self.active_screen == crate::screen_switch::PrimaryScreen::GuitarArticulation {
            self.enter_guitar_articulation();
        }
    }

    /// 画面を離れる・overlay へ音源を明け渡すときに、鳴っている演奏を止める。
    pub(in crate::tui) fn stop_guitar_articulation(&mut self) {
        if let Some(sender) = &self.mml_overlay_sender {
            sender.stop();
        }
    }
}
