use super::*;

#[test]
fn serialize_patches_dirs_line_escapes_single_quotes() {
    let line = serialize_patches_dirs_line(&[
        "/home/o'connor/.local/share/surge-data/patches_factory".to_string(),
        "/home/o'connor/.local/share/surge-data/patches_3rdparty".to_string(),
    ]);

    let toml_str = format!(
        r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
{line}
"#
    );

    let cfg: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(
        cfg.patches_dirs,
        Some(vec![
            "/home/o'connor/.local/share/surge-data/patches_factory".to_string(),
            "/home/o'connor/.local/share/surge-data/patches_3rdparty".to_string()
        ])
    );
}
