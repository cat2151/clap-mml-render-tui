use cmrt_offline_render::PluginEntries;

use std::sync::{Arc, Mutex};

use cmrt_tui_core::playback_session::PlaybackSession;

use super::notepad::{NotepadScreen, NotepadScreenParts};
use super::voicing::CatalogVoicings;
use super::{PatchLoadState, TuiApp};
use crate::config::Config;

mod grid_sequencer;

use grid_sequencer::{grid_session_from_history, grid_session_to_history};

struct LoadedSessionState {
    cursor: usize,
    lines: Vec<String>,
    active_screen: crate::screen_switch::PrimaryScreen,
    keyboard: crate::history::KeyboardSessionState,
    grid_sequencer_track_count: usize,
    grid_sequencer_chord_mode: bool,
    grid_sequencer: Option<crate::history::GridSequencerSessionState>,
    grid_sequencer_bpm: Option<f64>,
    loop_browser_bpm: Option<f64>,
    grid_sequencer_bpm_range: Option<[f64; 2]>,
    loop_browser_bpm_range: Option<[f64; 2]>,
    keyboard_note_guide_overlay_date: Option<String>,
    notepad_sound_check_guide_overlay_date: Option<String>,
    mml_overlay_patch: Option<String>,
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
        grid_sequencer_bpm,
        loop_browser_bpm,
        grid_sequencer_bpm_range,
        loop_browser_bpm_range,
        keyboard_note_guide_overlay_date,
        notepad_sound_check_guide_overlay_date,
        mml_overlay_patch,
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
        grid_sequencer_bpm,
        loop_browser_bpm,
        grid_sequencer_bpm_range,
        loop_browser_bpm_range,
        keyboard_note_guide_overlay_date,
        notepad_sound_check_guide_overlay_date,
        mml_overlay_patch,
    }
}

/// 保存済みの自動BPM範囲を復元する。未保存・不正なら `default_bpm` 固定の範囲。
fn bpm_range_from_history(
    saved: Option<[f64; 2]>,
    default_bpm: f64,
) -> cmrt_tui_core::bpm::BpmRange {
    saved
        .and_then(|[minimum, maximum]| cmrt_tui_core::bpm::BpmRange::new(minimum, maximum))
        .unwrap_or_else(|| cmrt_tui_core::bpm::BpmRange::fixed(default_bpm))
}

/// 既定のままの範囲は保存しない（history.json にキーを増やさない）。
fn bpm_range_to_history(range: cmrt_tui_core::bpm::BpmRange, default_bpm: f64) -> Option<[f64; 2]> {
    (range != cmrt_tui_core::bpm::BpmRange::fixed(default_bpm))
        .then(|| [range.minimum(), range.maximum()])
}

/// 復元直後の BPM モード。手動値が残っていればそれを、無ければ範囲から1つ引く。
fn restored_bpm_mode(
    manual: Option<f64>,
    range: cmrt_tui_core::bpm::BpmRange,
) -> cmrt_tui_core::bpm::BpmMode {
    cmrt_tui_core::bpm::BpmMode::from_saved(manual, range.sample())
}

/// パッチ一覧の非同期読み込みを開始し、共有状態ハンドルを返す。
///
/// adapterが生成したvoicingも同じfile cacheから復元し、一覧より先にmemoへ公開する。
/// TUI起動中に音色fileは走査しない。
fn spawn_patch_loader(
    cfg: &Config,
    catalog_voicings: CatalogVoicings,
    plugin_entries: PluginEntries,
) -> Arc<Mutex<PatchLoadState>> {
    // TUIはfile cacheを読むだけ。catalog走査とcache更新は明示的CLIだけが行う。
    let patch_load_state = Arc::new(Mutex::new(PatchLoadState::Loading));
    let state_bg = Arc::clone(&patch_load_state);
    let cfg = cfg.clone();
    std::thread::spawn(move || {
        let loading_started = std::time::Instant::now();
        match crate::patch_catalog_cache::load() {
            Ok(cache) => {
                let (mut snapshot, cached_voicings) = cache.into_parts();
                snapshot.rebuild_patch_roles(&crate::history::load_mml_patch_filter_presets());
                let restored_voicing_count = catalog_voicings.load_persisted(cached_voicings);
                crate::logging::global_log_sink(&format!(
                    "catalog-voicing: event=cache-restored count={restored_voicing_count}"
                ));
                let snapshot = Arc::new(snapshot);
                log_patch_load(&snapshot, loading_started.elapsed());
                *state_bg.lock().unwrap() = PatchLoadState::Ready(Arc::clone(&snapshot));

                if cfg.offline_render_backend == crate::config::OfflineRenderBackend::InProcess {
                    let loaded = snapshot
                        .catalog_plugins()
                        .iter()
                        .map(|plugin| cmrt_core::load_entry(&plugin.plugin_path))
                        .collect::<anyhow::Result<Vec<_>>>();
                    match loaded {
                        Ok(entries) => {
                            if let Err(error) = plugin_entries
                                .publish_owned(snapshot.catalog_plugins().to_vec(), entries)
                            {
                                crate::logging::global_log_sink(&format!(
                                    "patch-load: event=entry-publish-failed error=\"{error}\""
                                ));
                            }
                        }
                        Err(error) => {
                            let message = format!("PluginEntryの読み込みに失敗: {error:#}");
                            plugin_entries.publish_error(message.clone());
                            crate::logging::global_log_sink(&format!(
                                "patch-load: event=entry-load-failed error=\"{message}\""
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                let message = format!(
                    "patch catalog cacheを利用できません: {e:#}。`{}` を実行してください",
                    crate::patch_catalog_cache::BUILD_COMMAND
                );
                crate::logging::global_log_sink(&format!(
                    "patch-load: event=cache-error error=\"{message}\""
                ));
                plugin_entries.publish_error(message.clone());
                *state_bg.lock().unwrap() = PatchLoadState::Err(message);
            }
        }
    });
    patch_load_state
}

/// cache読み込み結果と、cache構築時に保存されたcatalog注記をlog.txtへ残す。
/// alternate screenを壊さないよう標準出力へは書かない。
fn log_patch_load(
    snapshot: &cmrt_tui_core::patch_load::PatchCatalogSnapshot,
    elapsed: std::time::Duration,
) {
    let names: Vec<&str> = snapshot
        .catalog_plugins()
        .iter()
        .map(|plugin| plugin.name.as_str())
        .collect();
    crate::logging::global_log_sink(&format!(
        "patch-load: event=cache-ready count={} ms={} plugins={}",
        snapshot.pairs().len(),
        elapsed.as_millis(),
        names.join(",")
    ));
    for note in snapshot.catalog_notes() {
        crate::logging::global_log_sink(&format!("patch-load: event=catalog-note note=\"{note}\""));
    }
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
    pub fn new(cfg: &'a Config, plugin_entries: PluginEntries) -> Self {
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
            grid_sequencer_bpm,
            loop_browser_bpm,
            grid_sequencer_bpm_range,
            loop_browser_bpm_range,
            keyboard_note_guide_overlay_date,
            notepad_sound_check_guide_overlay_date,
            mml_overlay_patch,
        } = load_initial_session_state();
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
        // MML オーバーレイも同じ SHM 接続を共有する（借りる instance は keyboard と同じ）。
        // sample rate は行ぜんぶを鳴らすときの live timeline を張るのに要る。
        let mml_overlay_sender = Some(super::mml_overlay::MmlOverlaySender::new(
            Arc::clone(&play_server),
            cfg.sample_rate,
        ));
        let keyboard_midi_sender = Some(super::keyboard::KeyboardMidiSender::new(
            Arc::clone(&play_server),
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
        // memo は一覧読み込みスレッドと `VoicingState` の両方が持つ（`Arc` 共有）。
        let catalog_voicings = CatalogVoicings::default();
        let patch_load_state =
            spawn_patch_loader(cfg, catalog_voicings.clone(), plugin_entries.clone());
        let grid_bpm_range =
            bpm_range_from_history(grid_sequencer_bpm_range, super::grid_sequencer::BPM);
        // 設定不足でカタログから外れたプラグインの案内。config は起動中に変わらないので
        // ここで 1 回だけ数え、音色選択を持つ画面すべてへ同じものを配る。
        let catalog_notes = Vec::new();

        Self {
            active_screen,
            screen_switch_menu: crate::screen_switch::ScreenSwitchMenu::default(),
            cfg: Arc::clone(&cfg_arc),
            plugin_entries: plugin_entries.clone(),
            notepad: NotepadScreen::new(NotepadScreenParts {
                lines,
                cursor,
                playback_session: playback_session.clone(),
                sound_check_guide_overlay_date: notepad_sound_check_guide_overlay_date,
                patch_load_state: Arc::clone(&patch_load_state),
                patch_phrase_store: crate::history::load_patch_phrase_store(),
                cfg: Arc::clone(&cfg_arc),
                plugin_entries,
                catalog_notes: catalog_notes.clone(),
            }),
            keyboard: super::keyboard::KeyboardScreen::new(
                keyboard_midi_sender,
                keyboard_state,
                super::keyboard::KeyboardMmlInput::default(),
                super::keyboard::KeyboardNoteGuide::new(keyboard_note_guide_overlay_date),
            ),
            loop_browser: {
                let mut screen = super::loop_browser::LoopBrowserScreen::default();
                let range = bpm_range_from_history(
                    loop_browser_bpm_range,
                    crate::loop_browser::time_stretch::TARGET_BPM,
                );
                screen.state.set_bpm_range(range);
                screen
                    .state
                    .set_bpm_mode(restored_bpm_mode(loop_browser_bpm, range));
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
                    bpm_mode: restored_bpm_mode(grid_sequencer_bpm, grid_bpm_range),
                    bpm_range: grid_bpm_range,
                    restored_session: grid_session_from_history(grid_sequencer),
                },
            ),
            mml_overlay: {
                let mut overlay = super::mml_overlay::MmlOverlay::default();
                overlay.set_restored_patch(mml_overlay_patch);
                overlay
            },
            mml_overlay_sender,
            voicing: super::voicing::VoicingState::with_catalog_voicings(
                crate::history::load_voicing_cache(),
                voicing_layers,
                voicing_source_refresh,
                super::voicing::VoicingPolicies::with_patch_load_state(
                    cfg,
                    Arc::clone(&patch_load_state),
                ),
                catalog_voicings,
            ),
            chord_progression_source,
            chord_catalog: cmrt_chord::ChordProgressionCatalog::default(),
            patch_load_state,
            catalog_notes,
            playback_session,
            play_server,
            dismissed_play_server_failure: None,
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
            grid_sequencer_bpm: self.grid_sequencer.bpm_mode().manual(),
            loop_browser_bpm: self.loop_browser.state.bpm_mode().manual(),
            grid_sequencer_bpm_range: bpm_range_to_history(
                self.grid_sequencer.bpm_range(),
                super::grid_sequencer::BPM,
            ),
            loop_browser_bpm_range: bpm_range_to_history(
                self.loop_browser.state.bpm_range(),
                crate::loop_browser::time_stretch::TARGET_BPM,
            ),
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
            mml_overlay_patch: self.mml_overlay.patch().map(str::to_owned),
        });
    }

    pub(super) fn save_keyboard_note_guide_overlay_date(&self) {
        if let Some(local_date) = self.keyboard.note_guide.last_overlay_date() {
            let _ = crate::history::save_keyboard_note_guide_overlay_date(local_date);
        }
    }
}
