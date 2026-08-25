//! history ファイルの置き場所（パス解決とテスト用ディレクトリへの差し替え）のテスト。
//!
//! `SessionState` の直列化そのものは親の `tests.rs` が見る。ここは「どこへ置くか」だけ。

use super::assert_history_file_path;

#[test]
fn session_state_path_is_in_history_dir() {
    match crate::session_state_path() {
        None => { /* dirs 利用不可の環境ではスキップ */ }
        Some(path) => assert_history_file_path(&path, "history.json"),
    }
}

#[test]
fn daw_file_path_ends_with_daw_json() {
    // daw_file_path() が利用可能な環境では "daw.json" という名前で終わること
    if let Some(path) = crate::daw_file_path() {
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("daw.json"));
    }
}

#[test]
fn daw_file_path_same_dir_as_history_json() {
    // daw_file_path() は history.json と同じディレクトリに配置される
    let history_path = crate::session_state_path();
    let daw_path = crate::daw_file_path();
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
    let history_path = crate::session_state_path();
    let patch_history_path = crate::patch_phrase_store_path();
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
fn history_files_use_test_temp_dir_under_tests() {
    let session_path = crate::session_state_path().expect("session_state_path should be available");
    let daw_path = crate::daw_file_path().expect("daw_file_path should be available");

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

/// `set_local_dir_envs` は history パスと `CMRT_BASE_DIR` を同時に差し替える。
/// app 側 config パスの差し替えは app crate の `config::tests` が検証する。
#[test]
fn set_local_dir_envs_redirects_history_paths_and_cmrt_base_dir() {
    let tmp = std::env::temp_dir().join("cmrt_test_local_dir_redirects_all_paths");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = crate::test_support::set_local_dir_envs(&tmp);
        let app_dir = tmp.join("clap-mml-render-tui");

        assert_eq!(
            std::env::var_os("CMRT_BASE_DIR").map(std::path::PathBuf::from),
            Some(app_dir.clone())
        );
        assert_eq!(
            crate::daw_file_path().as_deref(),
            Some(app_dir.join("history").join("daw.json").as_path())
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}
