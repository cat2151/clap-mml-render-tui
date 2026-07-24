//! テスト時に history／config／core-lib 出力先を専用ディレクトリへ差し替えるヘルパ。
//!
//! `test-support` feature（および本 crate 自身の `cfg(test)`）でのみコンパイルされる。
//! app 側 `crate::test_utils` はここを再エクスポートして従来のパスを維持する。
//!
//! 差し替えは2系統ある。
//! - スレッドローカルの app ディレクトリ（`set_app_dir_for_current_thread`）
//! - 環境変数 `CMRT_BASE_DIR`（`cmrt_runtime` / 各ドメイン crate が参照する）
//!
//! [`set_local_dir_envs`] は両方を同時に設定し、スコープを抜けると元に戻す。

use std::cell::RefCell;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// 環境変数を書き換えるテスト間の排他ロック（プロセス全体で1つ）。
pub fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

thread_local! {
    static TEST_HISTORY_APP_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// 現在スレッドに設定された app ディレクトリ。
pub fn current_thread_app_dir() -> Option<PathBuf> {
    TEST_HISTORY_APP_DIR.with(|dir| dir.borrow().clone())
}

/// 現在スレッドの app ディレクトリを差し替え、直前の値を返す。
pub fn set_app_dir_for_current_thread(path: Option<PathBuf>) -> Option<PathBuf> {
    TEST_HISTORY_APP_DIR.with(|dir| dir.replace(path))
}

fn default_test_app_dir_path() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let unique = format!(
            "cmrt_test_process_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch in tests")
                .as_nanos()
        );
        let app_dir = std::env::temp_dir().join(unique).join(super::APP_DIR_NAME);
        std::fs::create_dir_all(app_dir.join(super::HISTORY_DIR_NAME)).ok();
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::env::set_var("CMRT_BASE_DIR", &app_dir);
        app_dir
    })
}

/// スレッドローカル設定が無いときに使う、テストプロセス共通の app ディレクトリ。
pub fn default_test_app_dir() -> Option<PathBuf> {
    Some(default_test_app_dir_path().clone())
}

/// スレッドローカル設定を優先し、無ければプロセス共通の app ディレクトリを返す。
pub fn test_app_dir_for_current_thread_or_default() -> Option<PathBuf> {
    current_thread_app_dir().or_else(default_test_app_dir)
}

pub struct TestEnvGuard {
    _lock: Option<MutexGuard<'static, ()>>,
    vars: Vec<(&'static str, Option<String>)>,
    previous_history_app_dir: Option<Option<PathBuf>>,
}

impl Drop for TestEnvGuard {
    fn drop(&mut self) {
        for (key, original) in self.vars.iter().rev() {
            match original {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        if let Some(previous_history_app_dir) = self.previous_history_app_dir.take() {
            set_app_dir_for_current_thread(previous_history_app_dir);
        }
    }
}

/// テストの永続化先を現在スレッド専用の app ディレクトリへ切り替える。
/// history / config / ログのパスと core-lib の出力ベースをまとめて隔離する。
pub fn set_local_dir_envs(base: &Path) -> TestEnvGuard {
    let app_dir = base.join(super::APP_DIR_NAME);
    let history_dir = app_dir.join(super::HISTORY_DIR_NAME);
    let lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::fs::create_dir_all(&history_dir).ok();
    let previous_history_app_dir = set_app_dir_for_current_thread(Some(app_dir.clone()));
    TestEnvGuard {
        _lock: Some(lock),
        vars: set_env_vars([("CMRT_BASE_DIR", &app_dir)]),
        previous_history_app_dir: Some(previous_history_app_dir),
    }
}

/// prod と同じ解決規則で組み立てた history.json の期待パス。
pub fn session_state_path_for_test() -> Option<PathBuf> {
    test_app_dir_for_current_thread_or_default()
        .map(|d| d.join(super::HISTORY_DIR_NAME).join("history.json"))
        .or_else(|| {
            dirs::config_local_dir().map(|d| {
                d.join(super::APP_DIR_NAME)
                    .join(super::HISTORY_DIR_NAME)
                    .join("history.json")
            })
        })
}

pub fn patch_phrase_store_path_for_test() -> Option<PathBuf> {
    test_app_dir_for_current_thread_or_default()
        .map(|d| d.join(super::HISTORY_DIR_NAME).join("patch_history.json"))
        .or_else(|| {
            dirs::config_local_dir().map(|d| {
                d.join(super::APP_DIR_NAME)
                    .join(super::HISTORY_DIR_NAME)
                    .join("patch_history.json")
            })
        })
}

fn legacy_path_for_test(file_name: &str) -> Option<PathBuf> {
    test_app_dir_for_current_thread_or_default()
        .map(|d| d.join(file_name))
        .or_else(|| dirs::data_local_dir().map(|d| d.join(super::APP_DIR_NAME).join(file_name)))
}

pub fn legacy_session_state_path_for_test() -> Option<PathBuf> {
    legacy_path_for_test("history.json")
}

pub fn legacy_daw_session_state_path_for_test() -> Option<PathBuf> {
    legacy_path_for_test("history_daw.json")
}

pub fn legacy_patch_phrase_store_path_for_test() -> Option<PathBuf> {
    legacy_path_for_test("patch_history.json")
}

pub fn legacy_daw_file_path_for_test() -> Option<PathBuf> {
    legacy_path_for_test("daw.json")
}

fn set_env_vars<'a, I, V>(vars: I) -> Vec<(&'static str, Option<String>)>
where
    I: IntoIterator<Item = (&'static str, V)>,
    V: AsRef<OsStr> + 'a,
{
    vars.into_iter()
        .map(|(key, value)| {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            (key, original)
        })
        .collect()
}
