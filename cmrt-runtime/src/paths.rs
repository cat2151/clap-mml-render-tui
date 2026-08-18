use std::path::PathBuf;

#[cfg(test)]
const APP_DIR_NAME: &str = "clap-mml-render-tui";

/// OS 標準の設定ディレクトリ内のアプリ設定ディレクトリを返す。
/// - Windows: %LOCALAPPDATA%\clap-mml-render-tui  (Local 側)
/// - Linux:   ~/.config/clap-mml-render-tui
/// - macOS:   ~/Library/Application Support/clap-mml-render-tui
///
/// 場所そのものは play server repo 側（[`cmrt_server_config::config_app_dir`]）が
/// 単一ソース。サーバーと TUI が同じ config.toml を読むため。
///
/// システムの設定ディレクトリが取得できない場合は `None` を返す。
///
/// テストのときだけ差し替えのフックを噛ませる。ここを丸ごと共有 crate の再エクスポートに
/// すると、この crate 自身のテストが実ユーザーの config.toml を触りうる。
pub fn config_app_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(app_dir) = test_config_app_dir() {
        return Some(app_dir);
    }

    cmrt_server_config::config_app_dir()
}

pub fn config_file_path() -> Option<PathBuf> {
    config_app_dir().map(|d| d.join("config.toml"))
}

/// DAW デバッグログ (`log/log.txt`) のパスを返す。
/// `config.toml` と同じ config_local_dir 配下に配置する。
pub fn log_file_path() -> Option<PathBuf> {
    config_app_dir().map(|d| d.join("log").join("log.txt"))
}

/// native render probe 専用ログのパスを返す。
/// 既存の DAW デバッグログとは分離し、同じ log ディレクトリへ配置する。
pub fn native_probe_log_file_path() -> Option<PathBuf> {
    config_app_dir().map(|d| d.join("log").join("native_probe.log"))
}

#[cfg(test)]
fn test_config_app_dir() -> Option<PathBuf> {
    std::env::var_os("CMRT_BASE_DIR")
        .map(PathBuf::from)
        .or_else(|| Some(default_test_app_dir_path().clone()))
}

#[cfg(test)]
fn default_test_app_dir_path() -> &'static PathBuf {
    use std::sync::OnceLock;

    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let unique = format!(
            "cmrt_runtime_test_process_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch in tests")
                .as_nanos()
        );
        let app_dir = std::env::temp_dir().join(unique).join(APP_DIR_NAME);
        std::fs::create_dir_all(&app_dir).ok();
        app_dir
    })
}
