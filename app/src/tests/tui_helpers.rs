use super::*;

impl TuiApp<'static> {
    pub(super) fn new_for_test(cfg: Config) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            mode: Mode::Normal,
            lines: vec![String::new()],
            cursor: 0,
            list_state,
            textarea: TextArea::default(),
            cfg: Arc::new(cfg),
            entry_ptr: 0,
            play_state: Arc::new(Mutex::new(PlayState::Idle)),
            playback_session: Arc::new(AtomicU64::new(0)),
            active_sink: Arc::new(Mutex::new(None)),
            audio_cache: Arc::new(Mutex::new(HashMap::new())),
            patch_load_state: Arc::new(Mutex::new(PatchLoadState::Ready(Vec::new()))),
            patch_all: Vec::new(),
            patch_query: String::new(),
            patch_filtered: Vec::new(),
            patch_cursor: 0,
            patch_list_state: ListState::default(),
            patch_phrase_store: crate::history::PatchPhraseStore::default(),
            patch_phrase_name: None,
            patch_phrase_history_cursor: 0,
            patch_phrase_favorites_cursor: 0,
            patch_phrase_history_state: ListState::default(),
            patch_phrase_favorites_state: ListState::default(),
            patch_phrase_focus: PatchPhrasePane::History,
            patch_phrase_store_dirty: false,
            update_available: Arc::new(AtomicBool::new(false)),
            is_daw_mode: false,
        }
    }
}

impl TuiApp<'static> {
    pub(super) fn is_current_playback_session(&self, session: u64) -> bool {
        Self::playback_session_is_current(&self.playback_session, session)
    }
}
