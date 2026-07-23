//! ドメイン crate 単体テスト用のヘルパ。
//!
//! 永続パス（`app_dir()`）をテスト専用ディレクトリへ差し替える。`app_dir()` は
//! `#[cfg(test)]` 時に `CMRT_BASE_DIR` を参照するため、ここで env をロック付きで設定する。

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// スコープを抜けると `CMRT_BASE_DIR` を元に戻すガード。
pub(crate) struct AppDirGuard {
    _lock: MutexGuard<'static, ()>,
    previous: Option<OsString>,
}

impl Drop for AppDirGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("CMRT_BASE_DIR", value),
            None => std::env::remove_var("CMRT_BASE_DIR"),
        }
    }
}

/// `base/clap-mml-render-tui` を app ディレクトリとして `CMRT_BASE_DIR` に設定する。
/// app 側 `set_local_dir_envs` と同じ配置になる。
pub(crate) fn set_app_dir_env(base: &Path) -> AppDirGuard {
    let lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let app_dir: PathBuf = base.join("clap-mml-render-tui");
    std::fs::create_dir_all(&app_dir).ok();
    let previous = std::env::var_os("CMRT_BASE_DIR");
    std::env::set_var("CMRT_BASE_DIR", &app_dir);
    AppDirGuard {
        _lock: lock,
        previous,
    }
}
