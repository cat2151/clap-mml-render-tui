use super::*;
use std::path::Path;

#[path = "history/storage_and_migration.rs"]
mod storage_and_migration;

fn assert_history_file_path(path: &Path, file_name: &str) {
    assert_eq!(
        path.file_name().and_then(|n| n.to_str()),
        Some(file_name),
        "history ファイル名が期待と異なる: {:?}",
        path
    );

    let history_dir = path
        .parent()
        .expect("history ファイルに親ディレクトリがない");
    assert_eq!(
        history_dir.file_name().and_then(|n| n.to_str()),
        Some("history"),
        "history ファイルの親ディレクトリ名が history ではない: {:?}",
        history_dir
    );

    let app_dir = history_dir
        .parent()
        .expect("history ディレクトリにアプリディレクトリがない");
    assert_eq!(
        app_dir.file_name().and_then(|n| n.to_str()),
        Some("clap-mml-render-tui"),
        "history ファイルのアプリディレクトリ名が clap-mml-render-tui ではない: {:?}",
        app_dir
    );
}

#[test]
fn session_state_default_cursor_is_zero() {
    let state = SessionState::default();
    assert_eq!(state.cursor, 0);
}

#[test]
fn session_state_default_lines_is_cde() {
    let state = SessionState::default();
    assert_eq!(state.lines, vec!["cde".to_string()]);
}

#[test]
fn session_state_default_is_daw_mode_is_false() {
    let state = SessionState::default();
    assert!(!state.is_daw_mode);
}

#[test]
fn session_state_serialize_deserialize() {
    let state = SessionState {
        cursor: 42,
        lines: vec!["abc".to_string(), "def".to_string()],
        is_daw_mode: false,
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.cursor, 42);
    assert_eq!(loaded.lines, vec!["abc".to_string(), "def".to_string()]);
    assert!(!loaded.is_daw_mode);
}

#[test]
fn session_state_serialize_deserialize_zero() {
    let state = SessionState {
        cursor: 0,
        lines: vec!["cde".to_string()],
        is_daw_mode: false,
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.cursor, 0);
    assert_eq!(loaded.lines, vec!["cde".to_string()]);
    assert!(!loaded.is_daw_mode);
}

#[test]
fn session_state_serialize_deserialize_is_daw_mode_true() {
    let state = SessionState {
        cursor: 1,
        lines: vec!["cde".to_string()],
        is_daw_mode: true,
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.cursor, 1);
    assert!(loaded.is_daw_mode);
}

#[test]
fn session_state_json_from_invalid_returns_default() {
    // 不正なJSONはデフォルト値を返す
    let result: SessionState = serde_json::from_str("not json").unwrap_or_default();
    assert_eq!(result.cursor, 0);
    assert_eq!(result.lines, vec!["cde".to_string()]);
    assert!(!result.is_daw_mode);
}

#[test]
fn session_state_json_missing_field_returns_default() {
    // cursor フィールドがない場合はデフォルト値を返す
    let result: SessionState = serde_json::from_str("{}").unwrap_or_default();
    assert_eq!(result.cursor, 0);
    assert_eq!(result.lines, vec!["cde".to_string()]);
    assert!(!result.is_daw_mode);
}

#[test]
fn session_state_json_missing_lines_uses_default() {
    // lines フィールドがない場合（旧形式の history.json）はデフォルト値 ["cde"] を返す
    let result: SessionState = serde_json::from_str(r#"{"cursor": 3}"#).unwrap();
    assert_eq!(result.cursor, 3);
    assert_eq!(result.lines, vec!["cde".to_string()]);
}

#[test]
fn session_state_json_missing_is_daw_mode_defaults_to_false() {
    // is_daw_mode フィールドがない場合（旧形式の history.json）はデフォルト値 false を返す
    let result: SessionState = serde_json::from_str(r#"{"cursor": 3, "lines": ["cde"]}"#).unwrap();
    assert_eq!(result.cursor, 3);
    assert!(!result.is_daw_mode);
}

#[test]
fn session_state_json_empty_lines_passes_through_serde() {
    // serde は "lines": [] を空配列のままデシリアライズする（serde デフォルトは適用されない）。
    // load_session_state() がこれを検知して default_lines() で補填する。
    let raw: SessionState = serde_json::from_str(r#"{"cursor": 2, "lines": []}"#).unwrap();
    assert!(raw.lines.is_empty(), "serde は空配列をそのまま通す");
}

#[test]
fn save_and_load_session_state_roundtrip() {
    // 実ユーザーデータディレクトリに影響しないよう、一時ファイルに直接書き込んで
    // JSON シリアライズ/デシリアライズの往復を検証する
    let tmp_path = std::env::temp_dir().join("cmrt_test_history_roundtrip.json");

    let state = SessionState {
        cursor: 7,
        lines: vec!["cde".to_string(), "fga".to_string()],
        is_daw_mode: false,
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    std::fs::write(&tmp_path, &json).unwrap();

    let read_back = std::fs::read_to_string(&tmp_path).unwrap();
    let loaded: SessionState = serde_json::from_str(&read_back).unwrap();
    std::fs::remove_file(&tmp_path).ok();

    assert_eq!(loaded.cursor, 7);
    assert_eq!(loaded.lines, vec!["cde".to_string(), "fga".to_string()]);
    assert!(!loaded.is_daw_mode);
}

#[test]
fn session_state_path_is_in_history_dir() {
    match super::session_state_path() {
        None => { /* dirs 利用不可の環境ではスキップ */ }
        Some(path) => assert_history_file_path(&path, "history.json"),
    }
}

#[test]
fn daw_file_path_ends_with_daw_json() {
    // daw_file_path() が利用可能な環境では "daw.json" という名前で終わること
    if let Some(path) = super::daw_file_path() {
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("daw.json"));
    }
}

#[test]
fn daw_file_path_same_dir_as_history_json() {
    // daw_file_path() は history.json と同じディレクトリに配置される
    let history_path = super::session_state_path();
    let daw_path = super::daw_file_path();
    // dirs が利用できない環境では両方 None になるのでスキップ。
    // 一方のみが None の場合はロジックのバグを示すため失敗させる。
    match (history_path, daw_path) {
        (None, None) => { /* dirs 利用不可の環境ではスキップ */ }
        (Some(h), Some(d)) => {
            assert_eq!(h.parent(), d.parent());
        }
        (Some(_), None) => panic!("session_state_path() は Some だが daw_file_path() は None"),
        (None, Some(_)) => panic!("daw_file_path() は Some だが session_state_path() は None"),
    }
}

#[test]
fn patch_phrase_store_path_same_dir_as_history_json() {
    let history_path = super::session_state_path();
    let patch_history_path = super::patch_phrase_store_path();
    match (history_path, patch_history_path) {
        (None, None) => { /* dirs 利用不可の環境ではスキップ */ }
        (Some(h), Some(p)) => {
            assert_eq!(h.parent(), p.parent());
            assert_history_file_path(&p, "patch_history.json");
        }
        (Some(_), None) => {
            panic!("session_state_path() は Some だが patch_phrase_store_path() は None")
        }
        (None, Some(_)) => {
            panic!("patch_phrase_store_path() は Some だが session_state_path() は None")
        }
    }
}

#[test]
fn save_and_load_session_state_roundtrip_daw_mode() {
    // DAW モードのセッション状態が正しく保存・復元されることを検証する
    let tmp_path = std::env::temp_dir().join("cmrt_test_history_roundtrip_daw.json");

    let state = SessionState {
        cursor: 0,
        lines: vec!["cde".to_string()],
        is_daw_mode: true,
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    std::fs::write(&tmp_path, &json).unwrap();

    let read_back = std::fs::read_to_string(&tmp_path).unwrap();
    let loaded: SessionState = serde_json::from_str(&read_back).unwrap();
    std::fs::remove_file(&tmp_path).ok();

    assert!(loaded.is_daw_mode);
}

#[test]
fn history_files_use_test_temp_dir_under_tests() {
    let session_path = super::session_state_path().expect("session_state_path should be available");
    let daw_path = super::daw_file_path().expect("daw_file_path should be available");

    assert!(
        session_path.starts_with(std::env::temp_dir()),
        "session_state_path should stay under a test-only temp dir: {}",
        session_path.display()
    );
    assert!(
        daw_path.starts_with(std::env::temp_dir()),
        "daw_file_path should stay under a test-only temp dir: {}",
        daw_path.display()
    );
}

#[test]
fn set_local_dir_envs_redirects_config_history_and_cmrt_base_dir() {
    let tmp = std::env::temp_dir().join("cmrt_test_local_dir_redirects_all_paths");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);
        let app_dir = tmp.join("clap-mml-render-tui");

        assert_eq!(
            std::env::var_os("CMRT_BASE_DIR").map(std::path::PathBuf::from),
            Some(app_dir.clone())
        );
        assert_eq!(
            crate::config::config_file_path().as_deref(),
            Some(app_dir.join("config.toml").as_path())
        );
        assert_eq!(
            super::daw_file_path().as_deref(),
            Some(app_dir.join("history").join("daw.json").as_path())
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}
