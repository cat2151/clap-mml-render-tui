use super::*;

#[test]
fn configured_editors_uses_app_default_when_unset() {
    let path = std::env::temp_dir().join(format!(
        "cmrt-app-config-editors-{}-unset.toml",
        std::process::id()
    ));
    std::fs::write(&path, "sample_rate = 48000\n").unwrap();

    let editors = crate::config_editor::configured_editors(&path).unwrap();

    assert_eq!(
        editors,
        vec![
            "fresh".to_string(),
            "zed".to_string(),
            "code".to_string(),
            "edit".to_string(),
            "nano".to_string(),
            "vim".to_string()
        ]
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn configured_editors_uses_toml_value_when_set() {
    let path = std::env::temp_dir().join(format!(
        "cmrt-app-config-editors-{}-set.toml",
        std::process::id()
    ));
    std::fs::write(&path, r#"editors = ["code", "vim"]"#).unwrap();

    let editors = crate::config_editor::configured_editors(&path).unwrap();

    assert_eq!(editors, vec!["code".to_string(), "vim".to_string()]);
    std::fs::remove_file(path).unwrap();
}

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
