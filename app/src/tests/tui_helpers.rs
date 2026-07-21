use std::collections::{HashSet, VecDeque};

use ratatui::widgets::ListState;
use tui_textarea::TextArea;

use super::*;

impl TuiApp<'static> {
    pub(super) fn new_for_test(cfg: Config) -> Self {
        let render_queue = TuiRenderQueue::disabled_for_tests(
            cfg.offline_render_backend,
            cfg.effective_offline_render_workers(),
        );
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            mode: Mode::Normal,
            help_origin: Mode::Normal,
            editor: notepad_editor::NotepadEditorState::new(
                vec![String::new()],
                0,
                list_state,
                TextArea::default(),
            ),
            cfg: Arc::new(cfg),
            entry_ptr: 0,
            play_state: Arc::new(Mutex::new(PlayState::Idle)),
            playback_session: Arc::new(AtomicU64::new(0)),
            realtime_play_server: None,
            keyboard: KeyboardScreen::new(
                None,
                keyboard::KeyboardState::default(),
                keyboard::KeyboardMmlInput::default(),
                keyboard::KeyboardNoteGuide::new(None),
            ),
            notepad_sound_check_guide: crate::sound_check_guide::SoundCheckGuide::new(None),
            voicing_cache: crate::history::VoicingCache::default(),
            voicing_layers: crate::voicing_sources::VoicingLayers::default(),
            voicing_source_refresh: crate::voicing_sources::VoicingSourceRefresh::disabled(),
            active_offline_render_count: Arc::new(AtomicUsize::new(0)),
            render_queue,
            active_sink: Arc::new(Mutex::new(None)),
            audio_cache: Arc::new(Mutex::new(HashMap::new())),
            audio_cache_order: Arc::new(Mutex::new(VecDeque::new())),
            known_disk_cache_hashes: Arc::new(Mutex::new(HashSet::new())),
            patch_load_state: Arc::new(Mutex::new(PatchLoadState::Ready(Vec::new()))),
            random_patch_decks: crate::random::RandomIndexDecks::default(),
            patch_select: PatchSelectState::new(),
            notepad_history: NotepadHistoryState::new(),
            patch_phrase: PatchPhraseState::new(),
            patch_phrase_store: crate::history::PatchPhraseStore::default(),
            patch_phrase_store_dirty: false,
            is_daw_mode: false,
            startup_normal_cache_primed: false,
            loop_browser: loop_browser::LoopBrowserScreen::default(),
        }
    }

    pub(super) fn test_is_current_playback_session(&self, session: u64) -> bool {
        Self::playback_session_is_current(&self.playback_session, session)
    }

    pub(super) fn test_set_active_parallel_render_count(&self, count: usize) {
        self.active_offline_render_count
            .store(count, Ordering::Relaxed);
    }

    pub(super) fn test_set_render_job_status(
        &self,
        mml: impl Into<String>,
        status: Option<crate::tui::render_queue::TuiRenderJobStatus>,
    ) {
        self.render_queue.set_test_job_status(mml, status);
    }
}
