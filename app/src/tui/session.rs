use clack_host::prelude::PluginEntry;

use std::sync::{Arc, Mutex};

use cmrt_tui_core::playback_session::PlaybackSession;

use super::notepad::{NotepadScreen, NotepadScreenParts};
use super::{PatchLoadState, TuiApp};
use crate::config::Config;

struct LoadedSessionState {
    cursor: usize,
    lines: Vec<String>,
    active_screen: crate::screen_switch::PrimaryScreen,
    keyboard: crate::history::KeyboardSessionState,
    grid_sequencer_track_count: usize,
    grid_sequencer_chord_mode: bool,
    grid_sequencer: Option<crate::history::GridSequencerSessionState>,
    keyboard_note_guide_overlay_date: Option<String>,
    notepad_sound_check_guide_overlay_date: Option<String>,
}

/// 復元したセッションのカーソルを現在の行数に収まる範囲へ丸める。
///
/// `lines_len` は 1 以上であることを前提とする。
pub(super) fn clamp_session_cursor(cursor: usize, lines_len: usize) -> usize {
    debug_assert!(lines_len > 0, "session lines must not be empty");
    cursor.min(lines_len.saturating_sub(1))
}

fn load_initial_session_state() -> LoadedSessionState {
    // `lines` は常に1行以上を保持する（不変条件）。
    // load_session_state() は lines が空でないことを保証している。
    let crate::history::SessionState {
        cursor,
        lines,
        active_screen,
        keyboard,
        grid_sequencer_track_count,
        grid_sequencer_chord_mode,
        grid_sequencer,
        keyboard_note_guide_overlay_date,
        notepad_sound_check_guide_overlay_date,
    } = crate::history::load_session_state();
    let initial_cursor = clamp_session_cursor(cursor, lines.len());
    LoadedSessionState {
        cursor: initial_cursor,
        lines,
        active_screen,
        keyboard,
        grid_sequencer_track_count,
        grid_sequencer_chord_mode,
        grid_sequencer,
        keyboard_note_guide_overlay_date,
        notepad_sound_check_guide_overlay_date,
    }
}

fn grid_session_from_history(
    session: Option<crate::history::GridSequencerSessionState>,
) -> Option<super::grid_sequencer::GridSequencerSession> {
    let session = session.filter(|session| !session.instances.is_empty())?;
    let instances = session
        .instances
        .into_iter()
        .map(|instance| super::grid_sequencer::GridInstance {
            patch: instance.patch,
            lane_mode: match instance.lane_mode {
                crate::history::GridLaneModeState::Single => {
                    super::grid_sequencer::GridLaneMode::Single
                }
                crate::history::GridLaneModeState::BassOctave2 => {
                    super::grid_sequencer::GridLaneMode::BassOctave2
                }
                crate::history::GridLaneModeState::ChordVoices4 => {
                    super::grid_sequencer::GridLaneMode::ChordVoices4
                }
            },
            voicing_rotation: instance.voicing_rotation,
            lanes: instance
                .lanes
                .into_iter()
                .map(|lane| super::grid_sequencer::GridLane {
                    base_note: lane.base_note,
                    pattern: super::grid_sequencer::NotePattern::from_steps(
                        lane.note_steps.into_iter().map(history_note_step_to_domain),
                    ),
                })
                .collect(),
        })
        .collect();
    let pattern_evolution = match session.pattern_evolution {
        crate::history::GridPatternEvolutionState::Auto => {
            super::grid_sequencer::PatternEvolution::Auto
        }
        crate::history::GridPatternEvolutionState::Hold => {
            super::grid_sequencer::PatternEvolution::Hold
        }
    };
    Some(super::grid_sequencer::GridSequencerSession::new(
        instances,
        pattern_evolution,
    ))
}

fn history_note_step_to_domain(
    step: crate::history::GridNoteStepState,
) -> super::grid_sequencer::NoteStep {
    match step {
        crate::history::GridNoteStepState::Rest => super::grid_sequencer::NoteStep::Rest,
        crate::history::GridNoteStepState::Attack => super::grid_sequencer::NoteStep::Attack,
        crate::history::GridNoteStepState::Tie => super::grid_sequencer::NoteStep::Tie,
    }
}

fn domain_note_step_to_history(
    step: &super::grid_sequencer::NoteStep,
) -> crate::history::GridNoteStepState {
    match step {
        super::grid_sequencer::NoteStep::Rest => crate::history::GridNoteStepState::Rest,
        super::grid_sequencer::NoteStep::Attack => crate::history::GridNoteStepState::Attack,
        super::grid_sequencer::NoteStep::Tie => crate::history::GridNoteStepState::Tie,
    }
}

fn grid_session_to_history(
    session: Option<super::grid_sequencer::GridSequencerSession>,
) -> Option<crate::history::GridSequencerSessionState> {
    session.map(|session| crate::history::GridSequencerSessionState {
        instances: session
            .instances
            .into_iter()
            .map(|instance| crate::history::GridSequencerInstanceState {
                patch: instance.patch,
                lane_mode: match instance.lane_mode {
                    super::grid_sequencer::GridLaneMode::Single => {
                        crate::history::GridLaneModeState::Single
                    }
                    super::grid_sequencer::GridLaneMode::BassOctave2 => {
                        crate::history::GridLaneModeState::BassOctave2
                    }
                    super::grid_sequencer::GridLaneMode::ChordVoices4 => {
                        crate::history::GridLaneModeState::ChordVoices4
                    }
                },
                voicing_rotation: instance.voicing_rotation,
                lanes: instance
                    .lanes
                    .into_iter()
                    .map(|lane| crate::history::GridSequencerLaneState {
                        base_note: lane.base_note,
                        note_steps: lane
                            .pattern
                            .steps()
                            .iter()
                            .map(domain_note_step_to_history)
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
        pattern_evolution: match session.pattern_evolution {
            super::grid_sequencer::PatternEvolution::Auto => {
                crate::history::GridPatternEvolutionState::Auto
            }
            super::grid_sequencer::PatternEvolution::Hold => {
                crate::history::GridPatternEvolutionState::Hold
            }
        },
    })
}

/// パッチ一覧の非同期読み込みを開始し、共有状態ハンドルを返す。
fn spawn_patch_loader(cfg: &Config) -> Arc<Mutex<PatchLoadState>> {
    // パッチリストはバックグラウンドスレッドで収集する。
    // 起動時の同期スキャンによる遅延を避けるため。
    let patch_load_state = Arc::new(Mutex::new(PatchLoadState::Loading));
    let state_bg = Arc::clone(&patch_load_state);
    let cfg = cfg.clone();
    std::thread::spawn(move || match crate::patches::collect_patch_pairs(&cfg) {
        Ok(pairs) => {
            *state_bg.lock().unwrap() = PatchLoadState::Ready(pairs);
        }
        Err(e) => {
            *state_bg.lock().unwrap() = PatchLoadState::Err(e.to_string());
        }
    });
    patch_load_state
}

/// realtime play server をバックグラウンドで先行起動する。
///
/// keyboard / grid sequencer が使うサーバーは CLAP インスタンスを最大16個作るため
/// 起動に数秒かかる。画面へ入ってから起動すると、その待ち時間がまるごと
/// 「音が鳴るまでの時間」になるので、app 起動直後に済ませてしまう。
/// `ensure_started` は Mutex 下でポート開通を先に確認するため、画面側の起動要求と
/// 同時に走っても二重に spawn されない。
fn spawn_play_server_prewarm(
    play_server: &Arc<crate::realtime_play::RealtimePlayServerSupervisor>,
) {
    let play_server = Arc::clone(play_server);
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let result = play_server.ensure_started_for_fast_midi();
        crate::logging::global_log_sink(&format!(
            "play-server: prewarm ms={} result={}",
            started.elapsed().as_millis(),
            match &result {
                Ok(()) => "ok".to_string(),
                Err(error) => format!("error \"{error:#}\""),
            }
        ));
    });
}

impl<'a> TuiApp<'a> {
    pub fn new(cfg: &'a Config, entry: Option<&'a PluginEntry>) -> Self {
        crate::logging::install_native_probe_logger();
        let cfg_arc = Arc::new(cfg.clone());
        let LoadedSessionState {
            cursor,
            lines,
            active_screen,
            keyboard,
            grid_sequencer_track_count,
            grid_sequencer_chord_mode,
            grid_sequencer,
            keyboard_note_guide_overlay_date,
            notepad_sound_check_guide_overlay_date,
        } = load_initial_session_state();
        let entry_ptr = entry
            .map(|entry| entry as *const PluginEntry as usize)
            .unwrap_or(0);
        let play_server = Arc::new(
            crate::realtime_play::RealtimePlayServerSupervisor::with_live_instance_count(
                cfg_arc.as_ref(),
                // 1 トラックにつき bank 2 本。grid sequencer の chord mode が、
                // 鳴っている bank の裏でもう一方へ次の patch を先読みするため。
                crate::realtime_play::server_instance_count(grid_sequencer_track_count),
            ),
        );
        if cfg_arc.realtime_play_server_prewarm {
            spawn_play_server_prewarm(&play_server);
        }
        let realtime_play_server =
            if cfg_arc.realtime_audio_backend == crate::config::RealtimeAudioBackend::PlayServer {
                Some(Arc::clone(&play_server))
            } else {
                None
            };
        let keyboard_state = super::keyboard::KeyboardState::from_session(keyboard);
        // keyboard と grid sequencer は supervisor が所有する1本のSHM接続を共有する。
        let grid_midi_sender = Some(super::grid_sequencer::GridMidiSender::new(Arc::clone(
            &play_server,
        )));
        let keyboard_midi_sender = Some(super::keyboard::KeyboardMidiSender::new(
            play_server,
            keyboard_state.buffer_multiplier(),
        ));
        let restore_keyboard = active_screen == crate::screen_switch::PrimaryScreen::Keyboard;
        let voicing_source_refresh = crate::voicing_sources::VoicingSourceRefresh::spawn(cfg);
        let voicing_layers = if restore_keyboard {
            voicing_source_refresh.load_for_keyboard()
        } else {
            crate::voicing_sources::VoicingLayers::default()
        };

        // コード進行カタログの取得はここで走らせておき、読むのは grid sequencer 画面へ
        // 入るとき（キャッシュがまだ無い初回だけ、そこで待たされる）。
        let chord_progression_source =
            crate::chord_progression_source::ChordProgressionSource::spawn(cfg);
        let playback_session = PlaybackSession::new(realtime_play_server);
        let patch_load_state = spawn_patch_loader(cfg);

        Self {
            active_screen,
            screen_switch_menu: crate::screen_switch::ScreenSwitchMenu::default(),
            cfg: Arc::clone(&cfg_arc),
            entry_ptr,
            notepad: NotepadScreen::new(NotepadScreenParts {
                lines,
                cursor,
                playback_session: playback_session.clone(),
                sound_check_guide_overlay_date: notepad_sound_check_guide_overlay_date,
                patch_load_state: Arc::clone(&patch_load_state),
                patch_phrase_store: crate::history::load_patch_phrase_store(),
                cfg: Arc::clone(&cfg_arc),
                entry_ptr,
            }),
            keyboard: super::keyboard::KeyboardScreen::new(
                keyboard_midi_sender,
                keyboard_state,
                super::keyboard::KeyboardMmlInput::default(),
                super::keyboard::KeyboardNoteGuide::new(keyboard_note_guide_overlay_date),
            ),
            loop_browser: {
                let mut screen = super::loop_browser::LoopBrowserScreen::default();
                screen.state.starting =
                    active_screen == crate::screen_switch::PrimaryScreen::LoopBrowser;
                screen
            },
            grid_sequencer: super::grid_sequencer::GridSequencerScreen::new_with(
                super::grid_sequencer::GridSequencerParts {
                    midi_sender: grid_midi_sender,
                    sample_rate: cfg.sample_rate,
                    buffer_frames: cfg.buffer_size,
                    track_count: grid_sequencer_track_count,
                    chord_enabled: grid_sequencer_chord_mode,
                    restored_session: grid_session_from_history(grid_sequencer),
                },
            ),
            voicing: super::voicing::VoicingState::new(
                crate::history::load_voicing_cache(),
                voicing_layers,
                voicing_source_refresh,
            ),
            chord_progression_source,
            chord_catalog: cmrt_chord::ChordProgressionCatalog::default(),
            patch_load_state,
            playback_session,
        }
    }

    pub(super) fn save_history_state(&self) {
        let _ = crate::history::save_session_state(&crate::history::SessionState {
            cursor: self.notepad.session_cursor(),
            lines: self.notepad.session_lines().to_vec(),
            active_screen: self.active_screen,
            keyboard: self.keyboard.state.session_state(),
            grid_sequencer_track_count: self.grid_sequencer.track_count(),
            grid_sequencer_chord_mode: self.grid_sequencer.chord_enabled(),
            grid_sequencer: grid_session_to_history(self.grid_sequencer.session_state()),
            keyboard_note_guide_overlay_date: self
                .keyboard
                .note_guide
                .last_overlay_date()
                .map(str::to_owned),
            notepad_sound_check_guide_overlay_date: self
                .notepad
                .sound_check_guide()
                .last_overlay_date()
                .map(str::to_owned),
        });
    }

    pub(super) fn save_keyboard_note_guide_overlay_date(&self) {
        if let Some(local_date) = self.keyboard.note_guide.last_overlay_date() {
            let _ = crate::history::save_keyboard_note_guide_overlay_date(local_date);
        }
    }
}
