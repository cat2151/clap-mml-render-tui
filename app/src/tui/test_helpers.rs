use super::*;

impl TuiApp<'static> {
    pub(super) fn new_for_test(cfg: Config) -> Self {
        // loop browser データ層（別 crate）へ、テストでも app ディレクトリ解決を注入する。
        crate::loop_browser::set_app_dir_resolver(crate::config::config_app_dir);
        let notepad = NotepadScreen::new_for_test(cfg.clone());
        // テストでは spawn しない。監督を作るだけではプロセスは立ち上がらない。
        let play_server = Arc::new(cmrt_realtime_play::RealtimePlayServerSupervisor::new(&cfg));
        let patch_plugins = cmrt_tui_core::patch_plugins::PatchPlugins::from_config(&cfg);
        let voicing_policies = voicing::VoicingPolicies::from_config(&cfg);
        Self {
            active_screen: crate::screen_switch::PrimaryScreen::Notepad,
            screen_switch_menu: crate::screen_switch::ScreenSwitchMenu::default(),
            cfg: Arc::new(cfg),
            plugin_entries: cmrt_offline_render::PluginEntries::none(),
            playback_session: notepad.playback_session().clone(),
            patch_load_state: Arc::clone(&notepad.patch_load_state),
            // 実マシンのインストール状況を読ませない。案内を見るテストは自分で入れる。
            catalog_notes: Vec::new(),
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
                voicing_policies,
            ),
            chord_progression_source:
                crate::chord_progression_source::ChordProgressionSource::disabled(),
            chord_catalog: ChordProgressionCatalog::default(),
            patch_plugins,
            play_server,
            dismissed_play_server_failure: None,
        }
    }
}
