//! Guitar Articulation 画面と TuiApp を接続する glue。
//!
//! 画面ロジック（状態・キー処理・描画・変換）は `cmrt-guitar-articulation` crate に閉じている。
//! 音を鳴らす手段（`MmlOverlaySender`）を持つのは app 側なので、画面が返した
//! 「この版を・この effect chain で鳴らしてほしい」をここで `play_line` へ渡す。
//!
//! 音色は、入った直後は読み込みの軽い Lite で鳴らし、裏で Full を先読みする。先読みが
//! 今の chain で読み終わったら、次の再生から Full にする（鳴っている演奏は張り直さない）。
//! 起動時の版が Full なら、入った時点で Full を読む（先読みはしない）。

use crossterm::event::KeyEvent;

use cmrt_effect_chain_select::chain_json;
use cmrt_guitar_articulation::TimedMidiEvent;
use cmrt_mml_overlay::line_play::{LinePerformance, LineProgram};
use cmrt_mml_overlay::{LivePatch, MmlOverlayPreload};

use crate::guitar_articulation_events::{play_log_line, play_note_log_line};
use crate::tui::guitar_articulation::{
    GuitarArticulationAction, GuitarArticulationSettings, Instrument, StartupInstrument, Take,
    FULL_PATCH,
};
use crate::tui::TuiApp;

impl TuiApp<'_> {
    /// キー 1 つを画面へ渡し、演奏の要求があればその場で鳴らす。
    /// [`GuitarArticulationAction::Quit`]（`q`）はランタイムがループを抜けるので、そのまま返す。
    pub(in crate::tui) fn handle_guitar_articulation_key_event(
        &mut self,
        key: KeyEvent,
    ) -> GuitarArticulationAction {
        let action = self.guitar_articulation.handle_key_event(key);
        match &action {
            GuitarArticulationAction::Play(take) => {
                let effect_chain = chain_json(self.guitar_articulation.sounding_effect_chain());
                self.play_guitar_articulation(*take, &effect_chain);
                self.preload_full_guitar(&effect_chain);
            }
            // 試聴の chain は確定していないので、Full をその chain で先読みしない。
            GuitarArticulationAction::PreviewEffectChain(chain) => {
                self.play_guitar_articulation(Take::Converted, &chain_json(chain));
            }
            GuitarArticulationAction::PlayNote { take, column } => {
                let events = self.guitar_articulation.column_events(*take);
                let effect_chain = chain_json(self.guitar_articulation.sounding_effect_chain());
                let patch = self
                    .sync_guitar_articulation_instrument(&effect_chain)
                    .patch();
                crate::logging::global_log_sink(&play_note_log_line(
                    patch, *take, *column, &events,
                ));
                self.play_guitar_articulation_events(events, &effect_chain, patch);
                self.preload_full_guitar(&effect_chain);
            }
            GuitarArticulationAction::Quit => self.stop_guitar_articulation(),
            GuitarArticulationAction::SaveHistory => self.save_guitar_articulation_history(),
            GuitarArticulationAction::SaveSettings => self.save_guitar_articulation_settings(),
            GuitarArticulationAction::OpenSampleMidiList => {
                self.open_guitar_articulation_sample_midi_list();
            }
            GuitarArticulationAction::LoadSampleMidi(path) => {
                self.load_guitar_articulation_sample_midi(path);
            }
            GuitarArticulationAction::PlaySampleMidi { note } => {
                self.play_guitar_articulation_sample_midi(*note);
            }
            GuitarArticulationAction::Continue => {}
        }
        action
    }

    /// 画面の履歴を専用 file へ書く。失敗してもログ 1 行に留め、画面は落とさない。
    pub(in crate::tui) fn save_guitar_articulation_history(&self) {
        if let Err(error) =
            crate::tui::guitar_articulation::save_history(self.guitar_articulation.history())
        {
            crate::logging::global_log_sink(&format!(
                "guitar-articulation: event=history-save-failed error=\"{error}\""
            ));
        }
    }

    /// 画面の設定を専用 file へ書く。失敗してもログ 1 行に留め、画面は落とさない。
    fn save_guitar_articulation_settings(&self) {
        let settings = GuitarArticulationSettings {
            startup_instrument: self.guitar_articulation.startup_instrument(),
        };
        if let Err(error) = crate::tui::guitar_articulation::save_settings(&settings) {
            crate::logging::global_log_sink(&format!(
                "guitar-articulation: event=settings-save-failed error=\"{error}\""
            ));
        }
    }

    /// その版のフレーズ全体を、MIDI filter を通さず `effect_chain` を掛けて 1 回だけ鳴らす。
    fn play_guitar_articulation(&mut self, take: Take, effect_chain: &str) {
        let events = self.guitar_articulation.events(take).to_vec();
        let patch = self
            .sync_guitar_articulation_instrument(effect_chain)
            .patch();
        crate::logging::global_log_sink(&play_log_line(
            patch,
            take,
            &events,
            self.guitar_articulation.mml(),
            self.guitar_articulation.rules(),
        ));
        self.play_guitar_articulation_events(events, effect_chain, patch);
    }

    /// イベント列を `patch` に `effect_chain` を掛けて 1 回だけ鳴らす。
    /// 前の演奏を止めるのは sender の責務（`play_line` は鳴っているものを止めてから積む）。
    pub(in crate::tui) fn play_guitar_articulation_events(
        &mut self,
        events: Vec<TimedMidiEvent>,
        effect_chain: &str,
        patch: &str,
    ) {
        // play server が上がっていなければ sender が無い。落とさず、何もしない。
        if let Some(sender) = &self.mml_overlay_sender {
            let loop_seconds = events.last().map_or(0.0, |event| event.seconds);
            sender.play_line(
                LivePatch::with_effect_chain(Some(patch), effect_chain),
                LineProgram::once(LinePerformance {
                    events,
                    loop_seconds,
                }),
            );
        }
    }

    /// 画面に入った時点で METAL-GTX を読み始め、Full でなければ裏で Full を先読みする。
    /// 読み込みは数秒かかるので、待ちは入った瞬間の中央 overlay（`sound_startup_overlay`）で見せる
    /// （先読みの待ちは見せない）。読み込み済みなら sender は何も読まず、overlay も出ない。
    /// MML が空なら既定の MML を鳴らす（sender が読み込みを待ってから鳴らす）。
    pub(in crate::tui) fn enter_guitar_articulation(&mut self) {
        let startup = self.guitar_articulation.startup_instrument();
        if startup == StartupInstrument::Full {
            self.guitar_articulation.set_instrument(Instrument::Full);
        }
        let effect_chain = chain_json(self.guitar_articulation.sounding_effect_chain());
        let patch = self
            .sync_guitar_articulation_instrument(&effect_chain)
            .patch();
        crate::logging::global_log_sink(&format!(
            "guitar-articulation: event=prepare startup={:?} patch={patch:?}",
            startup.label()
        ));
        if let Some(sender) = &self.mml_overlay_sender {
            sender.prepare(LivePatch::with_effect_chain(Some(patch), &effect_chain));
        }
        self.preload_full_guitar(&effect_chain);
        if let GuitarArticulationAction::Play(take) = self.guitar_articulation.enter() {
            self.play_guitar_articulation(take, &effect_chain);
        }
    }

    /// sender の先読みの状態から、いま鳴らす音色の段階と、読み込み中かを決めて画面へ渡す。
    ///
    /// 一度 Full になったら Lite へ戻さない（chain を替えても Full のまま）。Full にするのは、
    /// 先読みが Full を `effect_chain` で読み終えたときだけ。chain が違う Full を鳴らすと、
    /// 鳴らす instance で読み直しになりうるため。
    pub(in crate::tui) fn sync_guitar_articulation_instrument(
        &mut self,
        effect_chain: &str,
    ) -> Instrument {
        let status = self
            .mml_overlay_sender
            .as_ref()
            .map(cmrt_mml_overlay::MmlOverlaySender::status)
            .unwrap_or_default();
        self.guitar_articulation.set_sound_loading(
            status.is_loading() || matches!(status.preload(), Some(MmlOverlayPreload::Loading(_))),
        );
        if self.guitar_articulation.instrument() == Instrument::Full {
            return Instrument::Full;
        }
        let full = LivePatch::with_effect_chain(Some(FULL_PATCH), effect_chain);
        let instrument = match status.preload() {
            Some(MmlOverlayPreload::Ready(patch)) if *patch == full => Instrument::Full,
            Some(MmlOverlayPreload::Loading(patch)) if patch.patch() == Some(FULL_PATCH) => {
                Instrument::LiteLoadingFull
            }
            _ => Instrument::Lite,
        };
        self.guitar_articulation.set_instrument(instrument);
        instrument
    }

    /// まだ Full でなければ、Full を `effect_chain` で裏に読ませる。同じ先読みを重ねて頼んでも
    /// sender は読み直さない（読み込み中なら何もせず、読み終えていればそのまま使う）。
    pub(in crate::tui) fn preload_full_guitar(&self, effect_chain: &str) {
        if self.guitar_articulation.instrument() == Instrument::Full {
            return;
        }
        if let Some(sender) = &self.mml_overlay_sender {
            sender.preload(LivePatch::with_effect_chain(Some(FULL_PATCH), effect_chain));
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
