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
