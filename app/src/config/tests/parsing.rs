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
    assert_eq!(cfg.voicing_shared_source, DEFAULT_VOICING_SHARED_SOURCE);
    assert_eq!(cfg.voicing_override_source, DEFAULT_VOICING_OVERRIDE_SOURCE);
    assert_eq!(
        cfg.chord_progression_source,
        DEFAULT_CHORD_PROGRESSION_SOURCE
    );
    assert_eq!(
        cfg.chord_patch_categories,
        ["Keys", "Organs", "Pads", "Polysynths"]
    );
}

#[test]
fn config_parses_explicit_voicing_sources_and_allows_empty_values() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
voicing_shared_source = "data/shared.json"
voicing_override_source = ""
chord_progression_source = ""
chord_patch_categories = []
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert_eq!(cfg.voicing_shared_source, "data/shared.json");
    assert!(cfg.voicing_override_source.is_empty());
    assert!(cfg.chord_progression_source.is_empty());
    assert!(cfg.chord_patch_categories.is_empty());
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
