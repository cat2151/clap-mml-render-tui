use super::*;

#[test]
fn config_offline_render_workers_parses_explicit_value() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
offline_render_workers = 8
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    cfg.validate().unwrap();
    assert_eq!(cfg.offline_render_workers, 8);
}

#[test]
fn config_offline_render_backend_parses_render_server() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
offline_render_backend = "render_server"
offline_render_server_workers = 6
offline_render_server_port = 62153
offline_render_server_command = "cargo run -p clap-mml-render-server"
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    cfg.validate().unwrap();
    assert_eq!(
        cfg.offline_render_backend,
        OfflineRenderBackend::RenderServer
    );
    assert_eq!(cfg.offline_render_server_workers, 6);
    assert_eq!(cfg.effective_offline_render_workers(), 6);
    assert_eq!(cfg.offline_render_server_port, 62153);
    assert_eq!(
        cfg.offline_render_server_command,
        "cargo run -p clap-mml-render-server"
    );
}

#[test]
fn config_realtime_audio_backend_parses_play_server() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
realtime_audio_backend = "play_server"
realtime_play_server_port = 62154
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    cfg.validate().unwrap();
    assert_eq!(cfg.realtime_audio_backend, RealtimeAudioBackend::PlayServer);
    assert_eq!(cfg.realtime_play_server_port, 62154);
    // 実体の決め方は config.toml から外した（ADR 0017）。
    assert!(cfg.play_server_launch_override.is_none());
}

#[test]
fn config_effective_offline_render_workers_uses_backend_specific_value() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
offline_render_workers = 2
offline_render_server_workers = 4
"#;
    let mut cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.effective_offline_render_workers(), 2);

    cfg.offline_render_backend = OfflineRenderBackend::RenderServer;
    assert_eq!(cfg.effective_offline_render_workers(), 4);
}
