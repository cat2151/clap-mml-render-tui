use super::*;

impl TuiApp<'static> {
    pub(super) fn new_for_test(cfg: Config) -> Self {
        // loop browser データ層（別 crate）へ、テストでも app ディレクトリ解決を注入する。
        crate::loop_browser::set_app_dir_resolver(crate::config::config_app_dir);
        let notepad = NotepadScreen::new_for_test(cfg.clone());
        Self {
            active_screen: crate::screen_switch::PrimaryScreen::Notepad,
            screen_switch_menu: crate::screen_switch::ScreenSwitchMenu::default(),
            cfg: Arc::new(cfg),
            entry_ptr: 0,
            playback_session: notepad.playback_session().clone(),
            patch_load_state: Arc::clone(&notepad.patch_load_state),
            notepad,
            keyboard: KeyboardScreen::new(
                None,
                keyboard::KeyboardState::default(),
                keyboard::KeyboardMmlInput::default(),
                keyboard::KeyboardNoteGuide::new(None),
            ),
            loop_browser: loop_browser::LoopBrowserScreen::default(),
            mml_overlay: mml_overlay::MmlOverlay::default(),
            mml_overlay_sender: None,
            grid_sequencer: grid_sequencer::GridSequencerScreen::new(None),
            voicing: voicing::VoicingState::new(
                crate::history::VoicingCache::default(),
                crate::voicing_sources::VoicingLayers::default(),
                crate::voicing_sources::VoicingSourceRefresh::disabled(),
            ),
            chord_progression_source:
                crate::chord_progression_source::ChordProgressionSource::disabled(),
            chord_catalog: ChordProgressionCatalog::default(),
        }
    }
}
