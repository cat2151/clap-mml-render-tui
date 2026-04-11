use super::*;

impl TuiApp<'static> {
    pub(super) fn new_for_test(cfg: Config) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            mode: Mode::Normal,
            help_origin: Mode::Normal,
            lines: vec![String::new()],
            cursor: 0,
            list_state,
            textarea: TextArea::default(),
            cfg: Arc::new(cfg),
            entry_ptr: 0,
            play_state: Arc::new(Mutex::new(PlayState::Idle)),
            playback_session: Arc::new(AtomicU64::new(0)),
            active_offline_render_count: Arc::new(AtomicUsize::new(0)),
            active_sink: Arc::new(Mutex::new(None)),
            audio_cache: Arc::new(Mutex::new(HashMap::new())),
            patch_load_state: Arc::new(Mutex::new(PatchLoadState::Ready(Vec::new()))),
            patch_all: Vec::new(),
            patch_query: String::new(),
            patch_query_textarea: crate::text_input::new_single_line_textarea(""),
            patch_filtered: Vec::new(),
            patch_cursor: 0,
            patch_list_state: ListState::default(),
            patch_favorite_items: Vec::new(),
            patch_favorites_cursor: 0,
            patch_favorites_state: ListState::default(),
            patch_select_focus: PatchSelectPane::Patches,
            patch_select_filter_active: false,
            patch_select_sort_order: PatchSortOrder::Path,
            normal_page_size: 1,
            patch_select_page_size: 1,
            notepad_history_page_size: 1,
            patch_phrase_page_size: 1,
            patch_phrase_store: crate::history::PatchPhraseStore::default(),
            notepad_history_cursor: 0,
            notepad_favorites_cursor: 0,
            notepad_history_state: ListState::default(),
            notepad_favorites_state: ListState::default(),
            notepad_focus: PatchPhrasePane::History,
            notepad_query: String::new(),
            notepad_query_textarea: crate::text_input::new_single_line_textarea(""),
            notepad_filter_active: false,
            notepad_pending_delete: false,
            normal_pending_delete: false,
            yank_buffer: None,
            patch_phrase_name: None,
            patch_phrase_history_cursor: 0,
            patch_phrase_favorites_cursor: 0,
            patch_phrase_history_state: ListState::default(),
            patch_phrase_favorites_state: ListState::default(),
            patch_phrase_focus: PatchPhrasePane::History,
            patch_phrase_query: String::new(),
            patch_phrase_query_textarea: crate::text_input::new_single_line_textarea(""),
            patch_phrase_filter_active: false,
            patch_phrase_store_dirty: false,
            is_daw_mode: false,
        }
    }

    pub(super) fn test_is_current_playback_session(&self, session: u64) -> bool {
        Self::playback_session_is_current(&self.playback_session, session)
    }

    pub(super) fn test_set_active_parallel_render_count(&self, count: usize) {
        self.active_offline_render_count
            .store(count, Ordering::Relaxed);
    }
}
