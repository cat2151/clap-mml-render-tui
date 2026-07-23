use ratatui::widgets::ListState;
use tui_textarea::TextArea;

use super::*;

impl TuiApp<'static> {
    pub(super) fn new_for_test(cfg: Config) -> Self {
        // loop browser データ層（別 crate）へ、テストでも app ディレクトリ解決を注入する。
        crate::loop_browser::set_app_dir_resolver(crate::config::config_app_dir);
        let render_queue = TuiRenderQueue::disabled_for_tests(
            cfg.offline_render_backend,
            cfg.effective_offline_render_workers(),
        );
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            mode: Mode::Normal,
            help_origin: Mode::Normal,
            active_screen: crate::screen_switch::PrimaryScreen::Notepad,
            screen_switch_menu: crate::screen_switch::ScreenSwitchMenu::default(),
            editor: notepad_editor::NotepadEditorState::new(
                vec![String::new()],
                0,
                list_state,
                TextArea::default(),
            ),
            cfg: Arc::new(cfg),
            entry_ptr: 0,
            playback: playback_runtime::TuiPlaybackRuntime::new(
                None,
                render_queue,
                Arc::new(AtomicUsize::new(0)),
            ),
            keyboard: KeyboardScreen::new(
                None,
                keyboard::KeyboardState::default(),
                keyboard::KeyboardMmlInput::default(),
                keyboard::KeyboardNoteGuide::new(None),
            ),
            notepad_sound_check_guide: crate::sound_check_guide::SoundCheckGuide::new(None),
            voicing: voicing::VoicingState::new(
                crate::history::VoicingCache::default(),
                crate::voicing_sources::VoicingLayers::default(),
                crate::voicing_sources::VoicingSourceRefresh::disabled(),
            ),
            audio: audio_cache::NotepadAudioCache::new(),
            patch_load_state: Arc::new(Mutex::new(PatchLoadState::Ready(Vec::new()))),
            random_patch_decks: crate::random::RandomIndexDecks::default(),
            patch_select: PatchSelectState::new(),
            notepad_history: NotepadHistoryState::new(),
            patch_phrase: PatchPhraseState::new(),
            patch_phrase_store: crate::history::PatchPhraseStore::default(),
            patch_phrase_store_dirty: false,
            startup_normal_cache_primed: false,
            loop_browser: loop_browser::LoopBrowserScreen::default(),
        }
    }

    pub(super) fn test_is_current_playback_session(&self, session: u64) -> bool {
        Self::playback_session_is_current(&self.playback.playback_session, session)
    }

    pub(super) fn test_set_active_parallel_render_count(&self, count: usize) {
        self.playback
            .active_offline_render_count
            .store(count, Ordering::Relaxed);
    }

    pub(super) fn test_set_render_job_status(
        &self,
        mml: impl Into<String>,
        status: Option<crate::tui::render_queue::TuiRenderJobStatus>,
    ) {
        self.playback.render_queue.set_test_job_status(mml, status);
    }
}
