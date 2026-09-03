use std::sync::Arc;

use super::{
    grid::{build_grid_buffers_or_default, try_build_grid_buffers},
    offline_render_startup_log_line, realtime_audio_startup_log_line, realtime_audio_wiring,
};
use crate::{MEASURES, TRACKS};

fn config_with_realtime_backend(backend: &str) -> cmrt_runtime::Config {
    toml::from_str(&format!(
        r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi = "input.mid"
output_midi = "output.mid"
output_wav = "output.wav"
sample_rate = 44100
buffer_size = 512
realtime_audio_backend = "{backend}"
"#
    ))
    .unwrap()
}

fn supervisor_for_test() -> Arc<cmrt_realtime_play::RealtimePlayServerSupervisor> {
    // `new()` はプロセスを起こさない（ureq Agent と Mutex を作るだけ）ので、
    // どの backend の config で作っても同一性の検証には影響しない。
    Arc::new(cmrt_realtime_play::RealtimePlayServerSupervisor::new(
        &config_with_realtime_backend("cache_player"),
    ))
}

#[test]
fn try_build_grid_buffers_rejects_measure_overflow() {
    assert!(try_build_grid_buffers(2, usize::MAX).is_none());
}

#[test]
fn build_grid_buffers_or_default_falls_back_from_invalid_saved_size() {
    let buffers = build_grid_buffers_or_default(Some((usize::MAX, usize::MAX)));

    assert_eq!(buffers.tracks, TRACKS);
    assert_eq!(buffers.measures, MEASURES);
    assert_eq!(buffers.data.len(), TRACKS);
    assert_eq!(buffers.data[0].len(), MEASURES + 1);
}

#[test]
fn offline_render_startup_log_line_shows_backend_and_workers() {
    let cfg: cmrt_runtime::Config = toml::from_str(
        r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi = "input.mid"
output_midi = "output.mid"
output_wav = "output.wav"
sample_rate = 44100
buffer_size = 512
offline_render_backend = "render_server"
offline_render_server_workers = 4
"#,
    )
    .unwrap();

    assert_eq!(
        offline_render_startup_log_line(&cfg, cfg.effective_offline_render_workers()),
        "offline render: backend=render_server workers=4"
    );
}

#[test]
fn realtime_audio_startup_log_line_shows_backend() {
    let cfg: cmrt_runtime::Config = toml::from_str(
        r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi = "input.mid"
output_midi = "output.mid"
output_wav = "output.wav"
sample_rate = 44100
buffer_size = 512
realtime_audio_backend = "play_server"
"#,
    )
    .unwrap();

    assert_eq!(
        realtime_audio_startup_log_line(&cfg),
        "realtime audio: backend=play_server"
    );
}

#[test]
fn the_cache_player_backend_shares_one_supervisor_between_playback_and_the_mml_overlay() {
    let injected = supervisor_for_test();

    let wiring = realtime_audio_wiring(
        &config_with_realtime_backend("cache_player"),
        Some(Arc::clone(&injected)),
    );

    let live = wiring
        .live_play_server
        .expect("cache_player backend needs a supervisor");
    let overlay = wiring
        .mml_overlay
        .expect("the overlay keeps the injected one");
    // SHM の live 接続は 1 プロセス 1 本しか張れないので、同じ Arc でなければならない。
    assert!(Arc::ptr_eq(&live, &overlay));
    assert!(Arc::ptr_eq(&live, &injected));
}

#[test]
fn the_play_server_backend_still_builds_its_own_supervisor() {
    let injected = supervisor_for_test();

    let wiring = realtime_audio_wiring(
        &config_with_realtime_backend("play_server"),
        Some(Arc::clone(&injected)),
    );

    let live = wiring
        .live_play_server
        .expect("play_server backend needs a supervisor");
    let overlay = wiring
        .mml_overlay
        .expect("the overlay keeps the injected one");
    assert!(!Arc::ptr_eq(&live, &overlay));
    assert!(Arc::ptr_eq(&overlay, &injected));
}

#[test]
fn the_cache_player_backend_has_no_supervisor_when_none_is_injected() {
    let wiring = realtime_audio_wiring(&config_with_realtime_backend("cache_player"), None);

    assert!(wiring.live_play_server.is_none());
    assert!(wiring.mml_overlay.is_none());
}

/// backend を書いていない config（＝既定）では、DAW の演奏は live 経路へ行く。
///
/// rodio 経路を撤去したので、**既定のまま起動したユーザーが live 経路に載る**ことが
/// この機能の入口になる。TUI を起動しなくても、起動ログの文言と supervisor の配り先の
/// 2 つで「既定で live へ行く」ことを固定できる。
#[test]
fn a_config_without_a_backend_key_plays_through_the_live_cache_path() {
    let cfg: cmrt_runtime::Config = toml::from_str(
        r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi = "input.mid"
output_midi = "output.mid"
output_wav = "output.wav"
sample_rate = 44100
buffer_size = 512
"#,
    )
    .unwrap();

    assert_eq!(
        realtime_audio_startup_log_line(&cfg),
        "realtime audio: backend=cache_player"
    );

    let injected = supervisor_for_test();
    let wiring = realtime_audio_wiring(&cfg, Some(Arc::clone(&injected)));
    let live = wiring
        .live_play_server
        .expect("the default backend plays through the play server");
    assert!(Arc::ptr_eq(&live, &injected));
}
