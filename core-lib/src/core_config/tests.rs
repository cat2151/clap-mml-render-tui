use super::*;

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
    assert_eq!(core_cfg.output_wav, "output.wav");
    assert_eq!(core_cfg.buffer_size, 512);
}
