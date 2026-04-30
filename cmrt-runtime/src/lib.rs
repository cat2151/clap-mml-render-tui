use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_OFFLINE_RENDER_WORKERS: usize = 2;
pub const DEFAULT_OFFLINE_RENDER_SERVER_WORKERS: usize = 4;
pub const DEFAULT_OFFLINE_RENDER_SERVER_PORT: u16 = 62153;
pub const DEFAULT_REALTIME_PLAY_SERVER_PORT: u16 = 62154;
const MIN_OFFLINE_RENDER_WORKERS: usize = 1;
const MAX_OFFLINE_RENDER_WORKERS: usize = 16;
const APP_DIR_NAME: &str = "clap-mml-render-tui";

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OfflineRenderBackend {
    #[default]
    InProcess,
    RenderServer,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeAudioBackend {
    #[default]
    InProcess,
    PlayServer,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub plugin_path: String,
    pub input_midi: String,
    pub output_midi: String,
    pub output_wav: String,
    pub sample_rate: f64,
    pub buffer_size: usize,
    /// パッチ検索対象ディレクトリ一覧
    pub patches_dirs: Option<Vec<String>>,
    /// DAW のオフラインレンダリング同時実行数
    #[serde(default = "default_offline_render_workers")]
    pub offline_render_workers: usize,
    /// render-server backend のオフラインレンダリング同時実行数
    #[serde(default = "default_offline_render_server_workers")]
    pub offline_render_server_workers: usize,
    /// オフラインレンダリング backend
    #[serde(default)]
    pub offline_render_backend: OfflineRenderBackend,
    /// render-server backend が使う localhost port
    #[serde(default = "default_offline_render_server_port")]
    pub offline_render_server_port: u16,
    /// render-server backend 起動コマンド。空なら sibling executable / PATH を探す。
    #[serde(default)]
    pub offline_render_server_command: String,
    /// リアルタイム audio backend
    #[serde(default)]
    pub realtime_audio_backend: RealtimeAudioBackend,
    /// realtime play server backend が使う localhost port
    #[serde(default = "default_realtime_play_server_port")]
    pub realtime_play_server_port: u16,
    /// realtime play server backend 起動コマンド。空なら sibling executable / PATH を探す。
    #[serde(default)]
    pub realtime_play_server_command: String,
}

#[derive(Serialize)]
struct PatchesDirsToml<'a> {
    patches_dirs: &'a [String],
}

/// OS ごとのデフォルト plugin_path を返す。
/// 既知 OS でない場合は空文字を返す（ユーザーに設定を促す）。
#[cfg(target_os = "windows")]
pub fn default_plugin_path() -> &'static str {
    r"C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap"
}

#[cfg(target_os = "macos")]
pub fn default_plugin_path() -> &'static str {
    "/Library/Audio/Plug-Ins/CLAP/Surge XT.clap"
}

#[cfg(target_os = "linux")]
pub fn default_plugin_path() -> &'static str {
    "/usr/lib/clap/Surge XT.clap"
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn default_plugin_path() -> &'static str {
    ""
}

/// OS ごとのデフォルト patches_dirs を返す。
/// 既知 OS でない場合や取得できない場合は空配列を返す（ユーザーに設定を促す）。
#[cfg(target_os = "windows")]
pub fn default_patches_dirs() -> Vec<String> {
    vec![
        r"C:\ProgramData\Surge XT\patches_factory".to_string(),
        r"C:\ProgramData\Surge XT\patches_3rdparty".to_string(),
    ]
}

#[cfg(target_os = "macos")]
pub fn default_patches_dirs() -> Vec<String> {
    vec![
        "/Library/Application Support/Surge XT/patches_factory".to_string(),
        "/Library/Application Support/Surge XT/patches_3rdparty".to_string(),
    ]
}

#[cfg(target_os = "linux")]
pub fn default_patches_dirs() -> Vec<String> {
    dirs::data_dir()
        .map(|d| {
            vec![
                d.join("surge-data")
                    .join("patches_factory")
                    .to_string_lossy()
                    .into_owned(),
                d.join("surge-data")
                    .join("patches_3rdparty")
                    .to_string_lossy()
                    .into_owned(),
            ]
        })
        .unwrap_or_default()
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn default_patches_dirs() -> Vec<String> {
    Vec::new()
}

/// OS に応じたデフォルトの config.toml 内容を生成する。
pub fn default_config_content() -> String {
    default_config_content_with_app_settings("")
}

/// app 側の追加設定を含めたデフォルトの config.toml 内容を生成する。
pub fn default_config_content_with_app_settings(app_settings: &str) -> String {
    let plugin_path = default_plugin_path();
    let plugin_path_line = if plugin_path.is_empty() {
        // 未知の OS: ユーザーに設定を促すためコメントアウト状態で出力する
        "# plugin_path = \"\"  # ← お使いの CLAP プラグインのパスをここに設定してください"
            .to_string()
    } else {
        format!("plugin_path = '{plugin_path}'")
    };
    let patches_dirs = default_patches_dirs();
    let patches_dirs_line = if patches_dirs.is_empty() {
        // 未知の OS またはホームディレクトリが取得できない場合
        "# patches_dirs = []  # ← Surge XT の patches_factory / patches_3rdparty を設定してください"
            .to_string()
    } else {
        serialize_patches_dirs_line(&patches_dirs)
    };
    let app_settings = if app_settings.trim().is_empty() {
        String::new()
    } else {
        format!("{}\n", app_settings.trim_end())
    };
    format!(
        r#"# clap-mml-render-tui config
#
# 【必須】plugin_path にお使いの CLAP プラグインのパスを設定してください。
# 例 (Windows): plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'
# 例 (Linux):   plugin_path = '/usr/lib/clap/Surge XT.clap'
# 例 (macOS):   plugin_path = '/Library/Audio/Plug-Ins/CLAP/Surge XT.clap'
{plugin_path_line}

{app_settings}
input_midi  = "input.mid"
# output_midi, output_wav は自動的にシステム設定ディレクトリの clap-mml-render-tui/phrase/ または clap-mml-render-tui/daw/ に保存されます。
# 以下の値は内部的に使用されますが、実際の出力先は上記ディレクトリになります。
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512

# 【省略可】オフラインレンダリング同時実行数（1〜16）
# offline_render_backend = "in_process" のときに使います。
offline_render_workers = 2

# 【省略可】オフラインレンダリング backend
# in_process: 従来どおり cmrt 本体プロセス内でレンダリングします。
# render_server: 127.0.0.1 の render-server 子プロセスへ POST /render します。
offline_render_backend = "in_process"
offline_render_server_workers = 4
offline_render_server_port = 62153
offline_render_server_command = ""

# 【省略可】リアルタイム再生 backend
# in_process: 従来どおり cmrt 本体プロセス内で再生します。
# play_server: 127.0.0.1 の realtime play server 子プロセスへ POST /play します。
realtime_audio_backend = "in_process"
realtime_play_server_port = 62154
realtime_play_server_command = ""

# 【省略可】Surge XT パッチの検索対象ディレクトリ一覧（TUI / DAW の音色選択・ランダム音色で使う）
# 例 (Windows): patches_dirs = ['C:\ProgramData\Surge XT\patches_factory', 'C:\ProgramData\Surge XT\patches_3rdparty']
# 例 (Linux):   patches_dirs = ['/home/user/.local/share/surge-data/patches_factory', '/home/user/.local/share/surge-data/patches_3rdparty']
# 例 (macOS):   patches_dirs = ['/Library/Application Support/Surge XT/patches_factory', '/Library/Application Support/Surge XT/patches_3rdparty']
{patches_dirs_line}

"#,
    )
}

fn default_offline_render_workers() -> usize {
    DEFAULT_OFFLINE_RENDER_WORKERS
}

fn default_offline_render_server_workers() -> usize {
    DEFAULT_OFFLINE_RENDER_SERVER_WORKERS
}

fn default_offline_render_server_port() -> u16 {
    DEFAULT_OFFLINE_RENDER_SERVER_PORT
}

fn default_realtime_play_server_port() -> u16 {
    DEFAULT_REALTIME_PLAY_SERVER_PORT
}

/// `patches_dirs = [...]` の 1 行を安全な TOML 文字列として生成する。
///
/// パスに `'` や `\` が含まれても壊れないよう、手組みせず TOML シリアライズに任せる。
pub fn serialize_patches_dirs_line(patches_dirs: &[String]) -> String {
    toml::to_string(&PatchesDirsToml { patches_dirs })
        .unwrap_or_else(|_| "patches_dirs = []".to_string())
        .trim()
        .to_string()
}

/// OS 標準の設定ディレクトリ内のアプリ設定ディレクトリを返す。
/// - Windows: %LOCALAPPDATA%\clap-mml-render-tui  (Local 側)
/// - Linux:   ~/.config/clap-mml-render-tui
/// - macOS:   ~/Library/Application Support/clap-mml-render-tui
///
/// システムの設定ディレクトリが取得できない場合は `None` を返す。
pub fn config_app_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(app_dir) = test_config_app_dir() {
        return Some(app_dir);
    }

    dirs::config_local_dir().map(|d| d.join(APP_DIR_NAME))
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

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        Self::load_with_default_content(default_config_content())
    }

    pub fn load_with_default_content(default_content: impl Into<String>) -> anyhow::Result<Self> {
        let path = config_file_path().ok_or_else(|| {
            anyhow::anyhow!(
                "システムの設定ディレクトリが取得できません。HOME 環境変数などを確認してください。"
            )
        })?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = default_content.into();
        // create_new で排他的に作成することでレースコンディションを回避する。
        // AlreadyExists は既にファイルがある正常ケースなので無視する。
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write as _;
                match file.write_all(content.as_bytes()) {
                    Ok(_) => {
                        eprintln!(
                            "デフォルトの config.toml を作成しました: {}",
                            path.display()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "デフォルト config.toml の書き込みに失敗 ({}):\n--- 書き込もうとした内容 ---\n{}\n--- エラー: {}",
                            path.display(),
                            content,
                            e
                        );
                        return Err(anyhow::anyhow!(
                            "デフォルト config.toml の書き込みに失敗 ({}): {}",
                            path.display(),
                            e
                        ));
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => {
                eprintln!(
                    "config.toml の作成に失敗 ({}):\n--- 書き込もうとした内容 ---\n{}\n--- エラー: {}",
                    path.display(),
                    content,
                    e
                );
                return Err(anyhow::anyhow!(
                    "config.toml の作成に失敗 ({}): {}",
                    path.display(),
                    e
                ));
            }
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("config.toml が読めない ({}): {}", path.display(), e))?;
        let cfg: Self = toml::from_str(&text).map_err(|e| {
            anyhow::anyhow!("config.toml のパースに失敗 ({}): {}", path.display(), e)
        })?;
        cfg.validate()
            .map_err(|e| anyhow::anyhow!("config.toml の検証に失敗 ({}): {}", path.display(), e))?;
        Ok(cfg)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        validate_offline_render_workers("offline_render_workers", self.offline_render_workers)?;
        validate_offline_render_workers(
            "offline_render_server_workers",
            self.offline_render_server_workers,
        )?;
        if self.offline_render_server_port == 0 {
            anyhow::bail!("offline_render_server_port は 1〜65535 の範囲で設定してください");
        }
        if self.realtime_play_server_port == 0 {
            anyhow::bail!("realtime_play_server_port は 1〜65535 の範囲で設定してください");
        }
        Ok(())
    }

    pub fn effective_offline_render_workers(&self) -> usize {
        match self.offline_render_backend {
            OfflineRenderBackend::InProcess => self.offline_render_workers,
            OfflineRenderBackend::RenderServer => self.offline_render_server_workers,
        }
    }
}

impl OfflineRenderBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            OfflineRenderBackend::InProcess => "in_process",
            OfflineRenderBackend::RenderServer => "render_server",
        }
    }
}

impl RealtimeAudioBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            RealtimeAudioBackend::InProcess => "in_process",
            RealtimeAudioBackend::PlayServer => "play_server",
        }
    }
}

fn validate_offline_render_workers(name: &str, workers: usize) -> anyhow::Result<()> {
    if !(MIN_OFFLINE_RENDER_WORKERS..=MAX_OFFLINE_RENDER_WORKERS).contains(&workers) {
        anyhow::bail!(
            "{} は {}〜{} の範囲で設定してください（現在値: {}）",
            name,
            MIN_OFFLINE_RENDER_WORKERS,
            MAX_OFFLINE_RENDER_WORKERS,
            workers
        );
    }
    Ok(())
}

pub fn configured_patch_dirs(cfg: &Config) -> Vec<String> {
    cfg.patches_dirs
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|dir| !dir.trim().is_empty())
        .collect()
}

pub fn core_config_patch_root_dir(cfg: &Config) -> Option<String> {
    shared_patch_root_dir(&configured_patch_dirs(cfg))
}

pub fn shared_patch_root_dir(dirs: &[String]) -> Option<String> {
    let mut dir_paths = dirs.iter().map(PathBuf::from);
    let mut common = dir_paths.next()?;
    for dir in dir_paths {
        while !Path::new(&dir).starts_with(&common) {
            if !common.pop() {
                return None;
            }
        }
    }
    if common.as_os_str().is_empty() {
        return None;
    }
    Some(common.to_string_lossy().into_owned())
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

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;
