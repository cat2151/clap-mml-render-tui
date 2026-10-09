use cmrt_offline_render::EffectPlugins;

use std::sync::{Arc, Mutex};

use cmrt_tui_core::playback_session::PlaybackSession;

use super::notepad::{NotepadScreen, NotepadScreenParts};
use super::voicing::CatalogVoicings;
use super::{PatchLoadState, TuiApp};
use crate::config::Config;

mod drum_sequencer;
mod grid_sequencer;
mod persist;
mod restore;

use drum_sequencer::drum_session_from_history;
use grid_sequencer::grid_session_from_history;
use restore::{
    apply_startup_keyboard, bpm_range_from_history, load_initial_session_state,
    play_settings_from_history, restored_bpm_mode, LoadedSessionState,
};

/// 復元したセッションのカーソルを現在の行数に収まる範囲へ丸める。
///
/// `lines_len` は 1 以上であることを前提とする。
pub(super) fn clamp_session_cursor(cursor: usize, lines_len: usize) -> usize {
    debug_assert!(lines_len > 0, "session lines must not be empty");
    cursor.min(lines_len.saturating_sub(1))
}

/// パッチ一覧の非同期読み込みを開始し、共有状態ハンドルを返す。
///
/// adapterが生成したvoicingも同じfile cacheから復元し、一覧より先にmemoへ公開する。
/// TUI起動中に音色fileは走査しない。
fn spawn_patch_loader(
    catalog_voicings: CatalogVoicings,
    effect_plugins: EffectPlugins,
) -> Arc<Mutex<PatchLoadState>> {
    // TUIはfile cacheを読むだけ。catalog走査とcache更新は明示的CLIだけが行う。
    let patch_load_state = Arc::new(Mutex::new(PatchLoadState::Loading));
    let state_bg = Arc::clone(&patch_load_state);
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
                log_effect_catalog(&effect_plugins);
            }
            Err(e) => {
                let message = format!(
                    "patch catalog cacheを利用できません: {e:#}。`{}` を実行してください",
                    crate::patch_catalog_cache::BUILD_COMMAND
                );
                crate::logging::global_log_sink(&format!(
                    "patch-load: event=cache-error error=\"{message}\""
                ));
                *state_bg.lock().unwrap() = PatchLoadState::Err(message);
            }
        }
    });
    patch_load_state
}

/// effect の catalog を先に走査しておき、件数を log.txt へ残す。
/// ここで走査しないと DAW の EFFECT CHAIN overlay を最初に開くときにその分だけ待つ。
fn log_effect_catalog(effect_plugins: &EffectPlugins) {
    let Some(catalog) = effect_plugins.catalog() else {
        return;
    };
    crate::logging::global_log_sink(&format!(
        "effect-catalog: plugins={} presets={} skipped={}",
        catalog.plugins().len(),
        catalog.presets().len(),
        catalog.skipped().len()
    ));
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
    pub fn new(cfg: &'a Config, effect_plugins: EffectPlugins) -> anyhow::Result<Self> {
        Self::with_startup_keyboard(cfg, effect_plugins, None)
    }

    /// `startup_keyboard` が `Some` なら、保存済みの keyboard 状態をそれで置き換えて
    /// keyboard 画面で起動する（`cmrt kb`）。他の画面の復元は変わらない。
    pub fn with_startup_keyboard(
        cfg: &'a Config,
        effect_plugins: EffectPlugins,
        startup_keyboard: Option<crate::history::KeyboardSessionState>,
    ) -> anyhow::Result<Self> {
        let cfg_arc = Arc::new(cfg.clone());
        let LoadedSessionState {
            cursor,
            lines,
            mut active_screen,
            mut keyboard,
            grid_sequencer_track_count,
            grid_sequencer_chord_mode,
            grid_sequencer,
            drum_sequencer,
            grid_sequencer_bpm,
            loop_browser_bpm,
            grid_sequencer_bpm_range,
            loop_browser_bpm_range,
            keyboard_note_guide_overlay_date,
            notepad_sound_check_guide_overlay_date,
            mml_overlay_patch,
            chord_chart_patch,
            chord_chart_bass_enabled,
            chord_chart_bass_patch,
            chord_chart_query,
            chord_chart_bass_query,
            chord_chart_effect_chain,
            mml_overlay_play_settings,
        } = load_initial_session_state()?;
        apply_startup_keyboard(&mut active_screen, &mut keyboard, startup_keyboard);
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
        let keyboard_mml_input = super::keyboard::KeyboardMmlInput::restored(keyboard.mml.clone());
        let keyboard_effect_chain = keyboard.effect_chain.clone();
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
            cfg.sample_rate,
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
        let patch_load_state = spawn_patch_loader(catalog_voicings.clone(), effect_plugins.clone());
        let grid_bpm_range =
            bpm_range_from_history(grid_sequencer_bpm_range, super::grid_sequencer::BPM);
        // 設定不足でカタログから外れたプラグインの案内。config は起動中に変わらないので
        // ここで 1 回だけ数え、音色選択を持つ画面すべてへ同じものを配る。
        let catalog_notes = Vec::new();
        let notepad_effect_plugins = effect_plugins.clone();
        let grid_effect_plugins = effect_plugins.clone();
        let guitar_effect_plugins = effect_plugins.clone();
        let guitar_settings = super::guitar_articulation::load_settings();
        let keyboard_effect_plugins = effect_plugins.clone();

        Ok(Self {
            active_screen,
            screen_switch_menu: crate::screen_switch::ScreenSwitchMenu::default(),
            cfg: Arc::clone(&cfg_arc),
            effect_plugins,
            notepad: NotepadScreen::new(NotepadScreenParts {
                lines,
                cursor,
                playback_session: playback_session.clone(),
                sound_check_guide_overlay_date: notepad_sound_check_guide_overlay_date,
                patch_load_state: Arc::clone(&patch_load_state),
                patch_phrase_store: crate::history::load_patch_phrase_store(),
                cfg: Arc::clone(&cfg_arc),
                catalog_notes: catalog_notes.clone(),
                effect_plugins: notepad_effect_plugins,
            }),
            keyboard: super::keyboard::KeyboardScreen::new(
                keyboard_midi_sender,
                keyboard_state,
                keyboard_mml_input,
                super::keyboard::KeyboardNoteGuide::new(keyboard_note_guide_overlay_date),
            )
            .with_chord_progression_source(super::keyboard_glue::keyboard_catalog_source_from(
                chord_progression_source.clone(),
            ))
            .with_effect_plugins(keyboard_effect_plugins)
            .with_effect_chain(keyboard_effect_chain),
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
                    effect_plugins: grid_effect_plugins,
                },
            ),
            // kit は名前だけ戻り、catalog の読み込み後に照合する。再生は止まった状態で始める。
            drum_sequencer: drum_session_from_history(drum_sequencer),
            // 保存済みの曲。ネットワークには触らないので起動時に読んでよい
            // （この画面がネットワークを要するのはコード進行カタログだけ）。
            // 読めなかったとき（初回 / 壊れている）に曲をでっち上げるのはここではない。
            // 画面を最初に開いた時点で 1 つ抽選する（`ChordChartScreen::enter`）。
            chord_chart: {
                let mut screen =
                    super::chord_chart::ChordChartScreen::restored(super::chord_chart::load_song());
                // カタログは `g` / `r` を押した瞬間に初めて引く（DAW の chord wizard と
                // 同じ遅延クロージャ。ここで `catalog()` を呼ぶと起動が待たされる）。
                screen.set_chord_progression_source(
                    super::chord_chart_glue::chord_chart_catalog_source_from(
                        chord_progression_source.clone(),
                        crate::logging::global_log_sink,
                    ),
                );
                screen.set_bass_enabled(chord_chart_bass_enabled);
                screen
            },
            // preview はまだ 1 度も鳴らしていない。
            chord_chart_preview_command_id: None,
            deferred_chord_chart_preview: None,
            guitar_articulation:
                super::guitar_articulation::GuitarArticulationScreen::with_effect_plugins(
                    guitar_effect_plugins,
                )
                .with_history(super::guitar_articulation::load_history())
                .with_startup_instrument(guitar_settings.startup_instrument)
                .with_arp_materials(guitar_settings.arp_materials)
                .with_arp_material(guitar_settings.arp_material)
                .with_arp(guitar_settings.arp),
            guitar_articulation_playback: None,
            grid_history_preview: crate::daw::DawGridPreviewPlayer::new(Arc::clone(&cfg_arc)),
            mml_overlay: {
                let mut overlay = super::mml_overlay::MmlOverlay::default();
                overlay.set_restored_patch(mml_overlay_patch.clone());
                overlay.set_restored_play_settings(play_settings_from_history(
                    mml_overlay_play_settings,
                ));
                overlay
            },
            mml_overlay_owner: None,
            chord_chart_patch_select: None,
            chord_chart_effect_overlay: None,
            mml_overlay_patch,
            chord_chart_patch,
            chord_chart_bass_patch,
            chord_chart_query,
            chord_chart_bass_query,
            chord_chart_effect_chain_stages: chord_chart_effect_chain,
            chord_chart_auto_reverb_rules: Default::default(),
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
            // まだ何も鳴らそうとしていないので待ってもいない。
            sound_startup_wait: None,
            reported_sound_prepare_error: None,
        })
    }
}

#[cfg(test)]
mod tests;
