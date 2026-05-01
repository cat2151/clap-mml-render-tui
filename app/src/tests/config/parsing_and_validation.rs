use super::*;

#[test]
fn config_parse_valid_toml() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.plugin_path, "/usr/lib/clap/Surge XT.clap");
    assert_eq!(cfg.output_midi, "output.mid");
    assert_eq!(cfg.output_wav, "output.wav");
    assert!((cfg.sample_rate - 44100.0).abs() < f64::EPSILON);
    assert_eq!(cfg.buffer_size, 512);
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
fn config_parse_ignores_removed_patch_settings() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
patch_path = "/tmp/Pad 1.fxp"
random_patch = true
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    let core_cfg = core_config_from_config(&cfg);

    assert_eq!(cfg.plugin_path, "/usr/lib/clap/Surge XT.clap");
    assert!(core_cfg.patch_path.is_none());
    assert!(!core_cfg.random_patch);
}

#[test]
fn core_config_from_config_disables_random_patch() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
patches_dirs = ["/tmp/surge-data/patches_factory", "/tmp/surge-data/patches_3rdparty"]
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    let core_cfg = core_config_from_config(&cfg);
    assert!(
        !core_cfg.random_patch,
        "Config から生成した CoreConfig は常に random_patch=false にする"
    );
    assert_eq!(core_cfg.patches_dir.as_deref(), Some("/tmp/surge-data"));
}

#[test]
fn config_optional_patches_dirs_is_none_by_default() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert!(cfg.patches_dirs.is_none());
}

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
fn config_offline_render_workers_validation_rejects_out_of_range_values() {
    for workers in [0, 17] {
        let toml_str = format!(
            r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
offline_render_workers = {workers}
"#
        );
        let cfg: Config = toml::from_str(&toml_str).unwrap();
        assert!(
            cfg.validate().is_err(),
            "offline_render_workers={workers} は reject されるべき"
        );
    }
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
    cfg.validate().unwrap();
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

#[test]
fn config_offline_render_server_workers_validation_rejects_out_of_range_values() {
    for workers in [0, 17] {
        let toml_str = format!(
            r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
offline_render_server_workers = {workers}
"#
        );
        let cfg: Config = toml::from_str(&toml_str).unwrap();
        assert!(
            cfg.validate().is_err(),
            "offline_render_server_workers={workers} は reject されるべき"
        );
    }
}

#[test]
fn config_parse_ignores_removed_daw_size_settings() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
daw_tracks = 128
daw_measures = 256
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.plugin_path, "/usr/lib/clap/Surge XT.clap");
}
