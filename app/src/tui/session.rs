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
        keyboard_note_guide_overlay_date,
        notepad_sound_check_guide_overlay_date,
    } = crate::history::load_session_state();
    let initial_cursor = clamp_session_cursor(cursor, lines.len());
    LoadedSessionState {
        cursor: initial_cursor,
        lines,
        active_screen,
        keyboard,
        keyboard_note_guide_overlay_date,
        notepad_sound_check_guide_overlay_date,
    }
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

impl<'a> TuiApp<'a> {
    pub fn new(cfg: &'a Config, entry: Option<&'a PluginEntry>) -> Self {
        crate::logging::install_native_probe_logger();
        let cfg_arc = Arc::new(cfg.clone());
        let LoadedSessionState {
            cursor,
            lines,
            active_screen,
            keyboard,
            keyboard_note_guide_overlay_date,
            notepad_sound_check_guide_overlay_date,
        } = load_initial_session_state();
        let entry_ptr = entry
            .map(|entry| entry as *const PluginEntry as usize)
            .unwrap_or(0);
        let play_server = Arc::new(crate::realtime_play::RealtimePlayServerSupervisor::new(
            cfg_arc.as_ref(),
        ));
        let realtime_play_server =
            if cfg_arc.realtime_audio_backend == crate::config::RealtimeAudioBackend::PlayServer {
                Some(Arc::clone(&play_server))
            } else {
                None
            };
        let keyboard_state = super::keyboard::KeyboardState::from_session(keyboard);
        let keyboard_midi_sender = Some(super::keyboard::KeyboardMidiSender::new(
            play_server,
            keyboard_state.transport(),
            keyboard_state.buffer_multiplier(),
        ));
        let restore_keyboard = active_screen == crate::screen_switch::PrimaryScreen::Keyboard;
        let voicing_source_refresh = crate::voicing_sources::VoicingSourceRefresh::spawn(cfg);
        let voicing_layers = if restore_keyboard {
            voicing_source_refresh.load_for_keyboard()
        } else {
            crate::voicing_sources::VoicingLayers::default()
        };

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
            voicing: super::voicing::VoicingState::new(
                crate::history::load_voicing_cache(),
                voicing_layers,
                voicing_source_refresh,
            ),
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
