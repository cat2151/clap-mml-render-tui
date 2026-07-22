use super::*;

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
