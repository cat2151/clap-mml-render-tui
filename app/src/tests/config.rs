use super::*;

#[test]
fn config_file_path_ends_with_cmrt_config_toml() {
    if let Some(path) = config_file_path() {
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with("clap-mml-render-tui/config.toml")
                || path_str.ends_with(r"clap-mml-render-tui\config.toml"),
            "config_file_path が clap-mml-render-tui/config.toml で終わっていない: {}",
            path_str
        );
    }
    // dirs::config_dir() が None の環境ではテストをスキップする
}

#[test]
fn config_file_path_contains_cmrt_subdir() {
    if let Some(path) = config_file_path() {
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("clap-mml-render-tui"),
            "config_file_path に clap-mml-render-tui が含まれていない: {}",
            path_str
        );
    }
}

#[test]
fn log_file_path_ends_with_cmrt_log_txt() {
    if let Some(path) = log_file_path() {
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with("clap-mml-render-tui/log/log.txt")
                || path_str.ends_with(r"clap-mml-render-tui\log\log.txt"),
            "log_file_path が clap-mml-render-tui/log/log.txt で終わっていない: {}",
            path_str
        );
    }
}

#[test]
#[cfg(target_os = "windows")]
fn config_file_path_uses_local_not_roaming_on_windows() {
    if let Some(path) = config_file_path() {
        let path_str = path.to_string_lossy().to_lowercase();
        assert!(
            !path_str.contains("roaming"),
            "config_file_path が Roaming を使っている（Local を使うべき）: {}",
            path.display()
        );
        assert!(
            path_str.contains("local"),
            "config_file_path が Local を含んでいない: {}",
            path.display()
        );
    }
}

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
}

#[test]
fn default_config_content_uses_48000_sample_rate() {
    let content = default_config_content();

    assert!(
        content.contains("sample_rate = 48000"),
        "default config の sample_rate は 48000Hz であるべき: {}",
        content
    );
}

#[test]
fn default_config_content_uses_patches_dirs_key() {
    let content = default_config_content();

    assert!(
        content.contains("patches_dirs"),
        "default config は patches_dirs を案内するべき: {}",
        content
    );
}

#[test]
fn default_config_content_omits_removed_patch_path_key() {
    let content = default_config_content();

    assert!(
        !content.contains("patch_path"),
        "default config に削除済みの patch_path を残すべきではない: {}",
        content
    );
}

#[test]
fn default_config_content_preserves_windows_path_format() {
    let content = default_config_content();

    assert!(
        content.contains(
            r"# 例 (Windows): patches_dirs = ['C:\ProgramData\Surge XT\patches_factory', 'C:\ProgramData\Surge XT\patches_3rdparty']"
        ),
        "Windows の例示パスは単一バックスラッシュ表記を維持するべき: {}",
        content
    );
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
    let core_cfg = cmrt_core::CoreConfig::from(&cfg);

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
    let core_cfg = cmrt_core::CoreConfig::from(&cfg);
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
fn default_config_content_omits_removed_daw_size_keys() {
    let content = default_config_content();

    assert!(
        !content.contains("daw_tracks"),
        "default config に削除済みの daw_tracks を残すべきではない: {}",
        content
    );
    assert!(
        !content.contains("daw_measures"),
        "default config に削除済みの daw_measures を残すべきではない: {}",
        content
    );
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
