//! loop browser のデータ/永続化層。
//!
//! UI（app 側 `crate::tui::loop_browser`）に依存しない、メタデータ・トラックグリッド・
//! ランダムデッキ・WAV ライブラリ走査・タイムストレッチ再生の各モジュールを束ねる。
//!
//! 永続ファイルの配置先（app ディレクトリ）は app 側の解決関数に依存するため、
//! [`set_app_dir_resolver`] で注入する。未注入時は `cmrt_runtime` の解決へフォールバックする。
pub mod library;
pub mod metadata;
pub mod persisted;
pub mod random;
pub mod time_stretch;
pub mod track_grid;

#[cfg(test)]
pub(crate) mod test_support;

use std::path::PathBuf;
use std::sync::OnceLock;

type AppDirResolver = fn() -> Option<PathBuf>;

static APP_DIR_RESOLVER: OnceLock<AppDirResolver> = OnceLock::new();

/// app 起動時に一度だけ、app 側の app ディレクトリ解決関数（`crate::config::config_app_dir`）を注入する。
/// テスト環境の再ディレクトリ化を尊重するため、prod の直呼びではなくこの注入を経由する。
pub fn set_app_dir_resolver(resolver: AppDirResolver) {
    let _ = APP_DIR_RESOLVER.set(resolver);
}

/// 永続ファイルを配置する app ディレクトリを解決する。
pub(crate) fn app_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(base) = std::env::var_os("CMRT_BASE_DIR") {
        return Some(PathBuf::from(base));
    }
    APP_DIR_RESOLVER
        .get()
        .and_then(|resolver| resolver())
        .or_else(cmrt_runtime::config_app_dir)
}
