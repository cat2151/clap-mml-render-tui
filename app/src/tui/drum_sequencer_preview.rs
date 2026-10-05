//! Drum の単発イベント生成と、共有 sender への受け渡し。

use cmrt_chord::TimedMidiEvent;
use cmrt_drum_sequencer::{DrumSequencerScreen, DRUM_STEPS};
use cmrt_mml_overlay::{
    line_play::{LinePerformance, LineProgram},
    LivePatch,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;

use super::{drum_sequencer_glue::DrumSequencerAction, TuiApp};

const STEP_SECONDS: f64 = 0.125;
const NOTE_SECONDS: f64 = 2.0;

/// 現在の kit と matrix の積集合。各セルの off も独立して保持する。
fn preview_program(screen: &DrumSequencerScreen) -> LineProgram {
    let mut events = Vec::new();
    for &note in screen.notes() {
        for step in 0..DRUM_STEPS {
            if screen.cell_on(note, step) {
                let seconds = step as f64 * STEP_SECONDS;
                events.push(TimedMidiEvent {
                    seconds,
                    message: [0x90, note, 127],
                });
                events.push(TimedMidiEvent {
                    seconds: seconds + NOTE_SECONDS,
                    message: [0x80, note, 0],
                });
            }
        }
    }
    events.sort_by(|left, right| left.seconds.total_cmp(&right.seconds));
    let loop_seconds = events.last().map_or(0.0, |event| event.seconds);
    LineProgram::once(LinePerformance {
        events,
        loop_seconds,
    })
}

fn is_preview_key(key: KeyEvent) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        && (key.code == KeyCode::Char('P')
            || (key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::SHIFT)))
}

impl TuiApp<'_> {
    /// selector を最初に処理する。true は主画面での終了要求。
    pub(in crate::tui) fn dispatch_drum_sequencer_key_event(&mut self, key: KeyEvent) -> bool {
        if self.handle_drum_sequencer_key_event(key) != DrumSequencerAction::Unhandled {
            return false;
        }
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if is_preview_key(key) {
            self.play_drum_sequencer_preview();
        } else if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
            self.finish_drum_sequencer();
            return true;
        }
        false
    }

    fn play_drum_sequencer_preview(&mut self) {
        self.drum_sequencer.preview_error = None;
        let program = preview_program(&self.drum_sequencer.screen);
        // 未選択、不明/空一覧と、現在の kit に鳴らすセルが無い場合は停止だけ。
        if program.is_silent() {
            self.stop_drum_sequencer_preview();
            return;
        }
        let patch = LivePatch::new(self.drum_sequencer.screen.kit_name());
        let Some(sender) = &self.mml_overlay_sender else {
            self.drum_sequencer.preview_error = Some("音源の送信先を利用できません。".to_string());
            return;
        };
        self.drum_sequencer.preview_command = Some(sender.stop_and_play_line(patch, program));
    }

    pub(in crate::tui) fn stop_drum_sequencer_preview(&mut self) {
        self.drum_sequencer.preview_command = None;
        if let Some(sender) = &self.mml_overlay_sender {
            sender.stop();
        }
    }

    pub(in crate::tui) fn finish_drum_sequencer(&mut self) {
        self.stop_drum_sequencer_preview();
        self.drum_sequencer.close_selector();
    }

    pub(in crate::tui) fn draw_drum_sequencer(&mut self, frame: &mut Frame<'_>) {
        let status = self
            .mml_overlay_sender
            .as_ref()
            .map(|sender| sender.status());
        let command = self.drum_sequencer.preview_command;
        if let Some(error) = status
            .as_ref()
            .and_then(|status| command.and_then(|id| status.prepare_error_for(id)))
        {
            self.drum_sequencer.preview_error = Some(format!("鳴らせません: {error}"));
        }
        let loading = status
            .as_ref()
            .is_some_and(|status| Some(status.command_id()) == command && status.is_loading());
        let playing = status
            .as_ref()
            .and_then(|status| status.line_playback())
            .is_some_and(|playback| {
                Some(playback.command_id()) == command
                    && playback.is_sounding_at(std::time::Instant::now())
            });
        let text = self
            .drum_sequencer
            .preview_error
            .as_deref()
            .unwrap_or(if loading {
                "音源を読み込み中です。"
            } else if playing {
                "Preview: BPM 120 / 4/4 / 1回"
            } else {
                "BPM 120 / 4/4 / 16 steps"
            });
        cmrt_drum_sequencer::ui::draw_with_status(&mut self.drum_sequencer.screen, text, frame);
    }
}

#[cfg(test)]
mod tests;
