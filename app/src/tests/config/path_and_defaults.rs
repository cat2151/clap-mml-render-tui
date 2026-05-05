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
fn native_probe_log_file_path_ends_with_cmrt_native_probe_log() {
    if let Some(path) = native_probe_log_file_path() {
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with("clap-mml-render-tui/log/native_probe.log")
                || path_str.ends_with(r"clap-mml-render-tui\log\native_probe.log"),
            "native_probe_log_file_path が clap-mml-render-tui/log/native_probe.log で終わっていない: {}",
            path_str
        );
    }
}

#[test]
fn config_file_path_uses_test_temp_dir_under_tests() {
    let path = config_file_path().expect("test config path should be available");
    assert!(
        path.starts_with(std::env::temp_dir()),
        "config_file_path should stay under a test-only temp dir: {}",
        path.display()
    );
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
fn default_config_content_uses_config_editor_key() {
    let content = default_config_content();

    assert!(
        content.contains(r#"editors = ["fresh", "zed", "code", "edit", "nano", "vim"]"#),
        "default config は editors を案内するべき: {}",
        content
    );
}

#[test]
fn default_config_content_uses_offline_render_workers_key() {
    let content = default_config_content();

    assert!(
        content.contains("offline_render_workers = 2"),
        "default config は offline_render_workers を案内するべき: {}",
        content
    );
    assert!(
        content.contains("offline_render_server_workers = 4"),
        "default config は offline_render_server_workers を案内するべき: {}",
        content
    );
}

#[test]
fn default_config_content_uses_offline_render_backend_keys() {
    let content = default_config_content();

    assert!(
        content.contains("offline_render_backend = \"in_process\""),
        "default config は backend 既定値を案内するべき: {}",
        content
    );
    assert!(
        content.contains("offline_render_server_port = 62153"),
        "default config は render-server port を案内するべき: {}",
        content
    );
    assert!(
        content.contains("offline_render_server_command = \"\""),
        "default config は render-server command を案内するべき: {}",
        content
    );
    assert!(
        content.contains("realtime_audio_backend = \"in_process\""),
        "default config は realtime audio backend を案内するべき: {}",
        content
    );
    assert!(
        content.contains("realtime_play_server_port = 62154"),
        "default config は realtime play server port を案内するべき: {}",
        content
    );
    assert!(
        content.contains("realtime_play_server_command = \"\""),
        "default config は realtime play server command を案内するべき: {}",
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
