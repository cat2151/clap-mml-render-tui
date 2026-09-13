use super::*;

impl TuiApp<'static> {
    pub(super) fn new_for_test(cfg: Config) -> Self {
        // loop browser データ層（別 crate）へ、テストでも app ディレクトリ解決を注入する。
        crate::loop_browser::set_app_dir_resolver(crate::config::config_app_dir);
        let notepad = NotepadScreen::new_for_test(cfg.clone());
        // テストでは spawn しない。監督を作るだけではプロセスは立ち上がらない。
        let play_server = Arc::new(cmrt_realtime_play::RealtimePlayServerSupervisor::new(&cfg));
        let patch_load_state = Arc::clone(&notepad.patch_load_state);
        let voicing_policies =
            voicing::VoicingPolicies::with_patch_load_state(&cfg, Arc::clone(&patch_load_state));
        let cfg = Arc::new(cfg);
        Self {
            active_screen: crate::screen_switch::PrimaryScreen::Notepad,
            screen_switch_menu: crate::screen_switch::ScreenSwitchMenu::default(),
            cfg: Arc::clone(&cfg),
            plugin_entries: cmrt_offline_render::PluginEntries::none(),
            playback_session: notepad.playback_session().clone(),
            patch_load_state,
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
            mml_overlay_owner: None,
            mml_overlay_patch: None,
            chord_chart_patch: None,
            mml_overlay_sender: None,
            grid_sequencer: grid_sequencer::GridSequencerScreen::new(None),
            // テストでは実 `%LOCALAPPDATA%` の chord_chart.json を読ませない
            // （`load_song()` はファイルを読む）。代わりに section 1 つの曲を直に置く。
            //
            // `ChordChartScreen::restored(None)` にしないのは、画面へ入った瞬間に
            // カタログの抽選が走ってしまうため。自動抽選そのものを見るテストは
            // `tests/chord_chart_initial_song.rs` が明示的に組み立てる。
            chord_chart: chord_chart::ChordChartScreen::new(test_chord_chart_song()),
            chord_chart_preview_command_id: None,
            grid_history_preview: crate::daw::DawGridPreviewPlayer::disabled_for_tests(cfg),
            voicing: voicing::VoicingState::new(
                crate::history::VoicingCache::default(),
                crate::voicing_sources::VoicingLayers::default(),
                crate::voicing_sources::VoicingSourceRefresh::disabled(),
                voicing_policies,
            ),
            chord_progression_source:
                crate::chord_progression_source::ChordProgressionSource::disabled(),
            chord_catalog: ChordProgressionCatalog::default(),
            play_server,
            dismissed_play_server_failure: None,
            sound_startup_wait: None,
            reported_sound_prepare_error: None,
        }
    }
}

/// app のテストが使う chord chart の曲。section 1 つを 1 回だけ並べたもの。
///
/// 進行の文字列は**テスト用のダミー**（production はカタログから引くだけで、
/// コード進行を 1 つも持たない）。
pub(super) fn test_chord_chart_song() -> chord_chart::Song {
    let mut song = chord_chart::Song::empty();
    let id = song.push_section("A", "I-V-VIm-IV");
    song.arrangement = vec![id];
    song
}
