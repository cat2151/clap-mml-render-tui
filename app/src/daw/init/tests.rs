use super::{
    grid::{build_grid_buffers_or_default, try_build_grid_buffers},
    offline_render_startup_log_line, realtime_audio_startup_log_line,
};
use crate::daw::{MEASURES, TRACKS};

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
    let cfg: crate::config::Config = toml::from_str(
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
    let cfg: crate::config::Config = toml::from_str(
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
