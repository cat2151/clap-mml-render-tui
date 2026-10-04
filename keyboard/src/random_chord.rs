//! Keyboardのランダム進行。取得は有効化操作に限り、抽選には読み込んだcatalogを使う。

use std::{sync::Arc, time::Instant};

use cmrt_chord::{ChordProgressionCatalog, ChordProgressionPick};

use crate::{KeyboardScreen, PeriodicTick};

type CatalogSource = Arc<dyn Fn() -> ChordProgressionCatalog + Send + Sync>;

const PICK_ATTEMPTS: usize = 64;

#[derive(Default)]
pub(crate) struct RandomChordMode {
    pub(crate) enabled: bool,
    source: Option<CatalogSource>,
    catalog: ChordProgressionCatalog,
    pub(crate) error: Option<String>,
    pub(crate) pending_mml: Option<(Instant, String)>,
}

impl RandomChordMode {
    fn enable(&mut self) -> Result<ChordProgressionPick, String> {
        let source = self
            .source
            .as_ref()
            .ok_or_else(|| "コード進行カタログの供給元がありません".to_string())?;
        self.catalog = source();
        let pick = self.pick_next()?;
        self.enabled = true;
        self.error = None;
        Ok(pick)
    }

    pub(crate) fn pick_next(&self) -> Result<ChordProgressionPick, String> {
        if self.catalog.is_empty() {
            return Err("コード進行カタログを利用できません（空または読み込み失敗）".to_string());
        }
        self.catalog
            .pick_playable(PICK_ATTEMPTS)
            .ok_or_else(|| "再生できるコード進行を生成できません".to_string())
    }
}

impl KeyboardScreen<'_> {
    /// MIDIの予約を捨てる操作では、対応するMMLと進行の予約も一緒に捨てる。
    pub(crate) fn discard_random_chord_progression(&mut self) -> bool {
        self.random_chord.pending_mml = None;
        self.state.discard_scheduled_progression()
    }

    pub(crate) fn disable_random_chord_mode(&mut self, now: Instant) {
        self.apply_random_chord_progression(now);
        self.random_chord.enabled = false;
        if self.discard_random_chord_progression() {
            let ready = self.connection_status().phase.accepts_notes();
            let messages = self.state.restart_committed_progression(now, ready);
            self.periodic_timeline.request_cancel();
            if ready {
                self.send_after_cancel(messages);
            }
        }
    }

    pub(crate) fn cycle_keyboard_note_playback(&mut self, now: Instant) -> Vec<[u8; 3]> {
        self.apply_random_chord_progression(now);
        let discarded = self.discard_random_chord_progression();
        let messages = self.state.cycle_note_playback(now);
        if discarded {
            self.state.restart_committed_progression(now, true)
        } else {
            messages
        }
    }

    pub(crate) fn apply_random_chord_progression(&mut self, now: Instant) {
        self.state.apply_scheduled_progression(now);
        if self
            .random_chord
            .pending_mml
            .as_ref()
            .is_some_and(|(at, _)| *at <= now)
        {
            let (_, mml) = self.random_chord.pending_mml.take().unwrap();
            self.mml_input.set_confirmed(mml);
        }
    }

    pub(crate) fn poll_random_chord_tick(&mut self, horizon: Instant) -> Option<PeriodicTick> {
        let mode = &mut self.random_chord;
        self.state
            .poll_periodic_tick_with_progression(horizon, |at| {
                if !mode.enabled {
                    return None;
                }
                match mode.pick_next() {
                    Ok(pick) => {
                        mode.pending_mml = Some((at, format!("Key:{} {}", pick.key, pick.degrees)));
                        mode.error = None;
                        Some(pick.chords)
                    }
                    Err(error) => {
                        mode.error = Some(error);
                        None
                    }
                }
            })
    }

    /// appの共有供給元を遅延注入する。設定時にはcatalogを読み込まない。
    pub fn with_chord_progression_source(mut self, source: CatalogSource) -> Self {
        self.set_chord_progression_source(source);
        self
    }

    pub fn set_chord_progression_source(&mut self, source: CatalogSource) {
        self.random_chord = RandomChordMode {
            source: Some(source),
            ..RandomChordMode::default()
        };
    }

    pub fn random_chord_mode(&self) -> bool {
        self.random_chord.enabled
    }

    pub fn random_chord_error(&self) -> Option<&str> {
        self.random_chord.error.as_deref()
    }

    pub(crate) fn toggle_random_chord_mode(&mut self) {
        if self.random_chord.enabled {
            self.disable_random_chord_mode(Instant::now());
            return;
        }
        let pick = match self.random_chord.enable() {
            Ok(pick) => pick,
            Err(error) => {
                self.random_chord.error = Some(error);
                return;
            }
        };
        let ready = self.connection_status().phase.accepts_notes();
        let messages = self
            .state
            .replace_repeat_chords(pick.chords, Instant::now(), ready);
        self.mml_input
            .set_confirmed(format!("Key:{} {}", pick.key, pick.degrees));
        if ready {
            self.send_after_cancel(messages);
        }
    }
}

#[cfg(test)]
mod tests;
