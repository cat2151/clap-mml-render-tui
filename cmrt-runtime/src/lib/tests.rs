use super::*;

#[test]
fn config_parse_uses_runtime_defaults() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
"#;

    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert_eq!(cfg.offline_render_workers, DEFAULT_OFFLINE_RENDER_WORKERS);
    assert_eq!(
        cfg.offline_render_server_workers,
        DEFAULT_OFFLINE_RENDER_SERVER_WORKERS
    );
    assert_eq!(cfg.offline_render_backend, OfflineRenderBackend::InProcess);
    assert_eq!(
        cfg.offline_render_server_port,
        DEFAULT_OFFLINE_RENDER_SERVER_PORT
    );
    assert!(cfg.offline_render_server_command.is_empty());
    assert_eq!(cfg.realtime_audio_backend, RealtimeAudioBackend::InProcess);
    assert_eq!(
        cfg.realtime_play_server_port,
        DEFAULT_REALTIME_PLAY_SERVER_PORT
    );
    assert!(cfg.realtime_play_server_command.is_empty());
}

#[test]
fn default_config_content_contains_render_server_keys() {
    let content = default_config_content();

    assert!(content.contains("offline_render_workers = 2"));
    assert!(content.contains("offline_render_backend = \"in_process\""));
    assert!(content.contains("offline_render_server_workers = 4"));
    assert!(content.contains("offline_render_server_port = 62153"));
    assert!(content.contains("offline_render_server_command = \"\""));
    assert!(content.contains("realtime_audio_backend = \"in_process\""));
    assert!(content.contains("realtime_play_server_port = 62154"));
    assert!(content.contains("realtime_play_server_command = \"\""));
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
realtime_play_server_command = "clap-mml-realtime-play-server"
"#;

    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert_eq!(cfg.realtime_audio_backend, RealtimeAudioBackend::PlayServer);
    assert_eq!(cfg.realtime_play_server_port, 62154);
    assert_eq!(
        cfg.realtime_play_server_command,
        "clap-mml-realtime-play-server"
    );
}

#[test]
fn config_realtime_play_server_port_validation_rejects_zero() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
realtime_play_server_port = 0
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert!(cfg.validate().is_err());
}

#[test]
fn shared_patch_root_dir_returns_common_parent() {
    let dirs = vec![
        "/tmp/surge-data/patches_factory".to_string(),
        "/tmp/surge-data/patches_3rdparty".to_string(),
    ];

    let base = shared_patch_root_dir(&dirs);

    assert_eq!(base.as_deref(), Some("/tmp/surge-data"));
}

#[test]
fn core_config_patch_root_dir_uses_shared_patch_root() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
patches_dirs = ["/tmp/surge-data/patches_factory", "/tmp/surge-data/patches_3rdparty"]
"#;

    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert_eq!(
        core_config_patch_root_dir(&cfg).as_deref(),
        Some("/tmp/surge-data")
    );
}
