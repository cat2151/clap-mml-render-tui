use cmrt_core::CoreConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_OFFLINE_RENDER_WORKERS: usize = 4;
const MIN_OFFLINE_RENDER_WORKERS: usize = 1;
const MAX_OFFLINE_RENDER_WORKERS: usize = 16;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub plugin_path: String,
    #[allow(dead_code)]
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
}

#[derive(Serialize)]
struct PatchesDirsToml<'a> {
    patches_dirs: &'a [String],
}

/// OS ごとのデフォルト plugin_path を返す。
/// 既知 OS でない場合は空文字を返す（ユーザーに設定を促す）。
#[cfg(target_os = "windows")]
fn default_plugin_path() -> &'static str {
    r"C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap"
}

#[cfg(target_os = "macos")]
fn default_plugin_path() -> &'static str {
    "/Library/Audio/Plug-Ins/CLAP/Surge XT.clap"
}

#[cfg(target_os = "linux")]
fn default_plugin_path() -> &'static str {
    "/usr/lib/clap/Surge XT.clap"
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn default_plugin_path() -> &'static str {
    ""
}

/// OS ごとのデフォルト patches_dirs を返す。
/// 既知 OS でない場合や取得できない場合は空配列を返す（ユーザーに設定を促す）。
#[cfg(target_os = "windows")]
fn default_patches_dirs() -> Vec<String> {
    vec![
        r"C:\ProgramData\Surge XT\patches_factory".to_string(),
        r"C:\ProgramData\Surge XT\patches_3rdparty".to_string(),
    ]
}

#[cfg(target_os = "macos")]
fn default_patches_dirs() -> Vec<String> {
    vec![
        "/Library/Application Support/Surge XT/patches_factory".to_string(),
        "/Library/Application Support/Surge XT/patches_3rdparty".to_string(),
    ]
}

#[cfg(target_os = "linux")]
fn default_patches_dirs() -> Vec<String> {
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
fn default_patches_dirs() -> Vec<String> {
    Vec::new()
}

/// OS に応じたデフォルトの config.toml 内容を生成する。
fn default_config_content() -> String {
    let plugin_path = default_plugin_path();
    let plugin_path_line = if plugin_path.is_empty() {
        // 未知の OS: ユーザーに設定を促すためコメントアウト状態で出力する
        "# plugin_path = \"\"  # ← お使いの CLAP プラグインのパスをここに設定してください"
            .to_string()
    } else {
        format!("plugin_path = '{plugin_path}'", plugin_path = plugin_path)
    };
    let patches_dirs = default_patches_dirs();
    let patches_dirs_line = if patches_dirs.is_empty() {
        // 未知の OS またはホームディレクトリが取得できない場合
        "# patches_dirs = []  # ← Surge XT の patches_factory / patches_3rdparty を設定してください"
            .to_string()
    } else {
        serialize_patches_dirs_line(&patches_dirs)
    };
    format!(
        r#"# clap-mml-render-tui config
#
# 【必須】plugin_path にお使いの CLAP プラグインのパスを設定してください。
# 例 (Windows): plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'
# 例 (Linux):   plugin_path = '/usr/lib/clap/Surge XT.clap'
# 例 (macOS):   plugin_path = '/Library/Audio/Plug-Ins/CLAP/Surge XT.clap'
{plugin_path_line}

input_midi  = "input.mid"
# output_midi, output_wav は自動的にシステム設定ディレクトリの clap-mml-render-tui/phrase/ または clap-mml-render-tui/daw/ に保存されます。
# 以下の値は内部的に使用されますが、実際の出力先は上記ディレクトリになります。
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512

# 【省略可】DAW のオフラインレンダリング同時実行数（1〜16）
offline_render_workers = 4

# 【省略可】Surge XT パッチの検索対象ディレクトリ一覧（TUI / DAW の音色選択・ランダム音色で使う）
# 例 (Windows): patches_dirs = ['C:\ProgramData\Surge XT\patches_factory', 'C:\ProgramData\Surge XT\patches_3rdparty']
# 例 (Linux):   patches_dirs = ['/home/user/.local/share/surge-data/patches_factory', '/home/user/.local/share/surge-data/patches_3rdparty']
# 例 (macOS):   patches_dirs = ['/Library/Application Support/Surge XT/patches_factory', '/Library/Application Support/Surge XT/patches_3rdparty']
{patches_dirs_line}

"#,
        plugin_path_line = plugin_path_line,
        patches_dirs_line = patches_dirs_line
    )
}

fn default_offline_render_workers() -> usize {
    DEFAULT_OFFLINE_RENDER_WORKERS
}

/// `patches_dirs = [...]` の 1 行を安全な TOML 文字列として生成する。
///
/// パスに `'` や `\` が含まれても壊れないよう、手組みせず TOML シリアライズに任せる。
fn serialize_patches_dirs_line(patches_dirs: &[String]) -> String {
    toml::to_string(&PatchesDirsToml { patches_dirs })
        .unwrap_or_else(|_| "patches_dirs = []".to_string())
        .trim()
        .to_string()
}

/// OS 標準の設定ディレクトリ内の config.toml パスを返す。
/// - Windows: %LOCALAPPDATA%\clap-mml-render-tui\config.toml  (Local 側)
/// - Linux:   ~/.config/clap-mml-render-tui/config.toml
/// - macOS:   ~/Library/Application Support/clap-mml-render-tui/config.toml
///
/// システムの設定ディレクトリが取得できない場合は `None` を返す。
pub fn config_file_path() -> Option<PathBuf> {
    dirs::config_local_dir().map(|d| d.join("clap-mml-render-tui").join("config.toml"))
}

/// DAW デバッグログ (`log/log.txt`) のパスを返す。
/// `config.toml` と同じ config_local_dir 配下に配置する。
pub fn log_file_path() -> Option<PathBuf> {
    dirs::config_local_dir().map(|d| d.join("clap-mml-render-tui").join("log").join("log.txt"))
}

/// native render probe 専用ログのパスを返す。
/// 既存の DAW デバッグログとは分離し、同じ log ディレクトリへ配置する。
pub fn native_probe_log_file_path() -> Option<PathBuf> {
    dirs::config_local_dir().map(|d| {
        d.join("clap-mml-render-tui")
            .join("log")
            .join("native_probe.log")
    })
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = config_file_path().ok_or_else(|| {
            anyhow::anyhow!(
                "システムの設定ディレクトリが取得できません。HOME 環境変数などを確認してください。"
            )
        })?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = default_config_content();
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

    fn validate(&self) -> anyhow::Result<()> {
        if !(MIN_OFFLINE_RENDER_WORKERS..=MAX_OFFLINE_RENDER_WORKERS)
            .contains(&self.offline_render_workers)
        {
            anyhow::bail!(
                "offline_render_workers は {}〜{} の範囲で設定してください（現在値: {}）",
                MIN_OFFLINE_RENDER_WORKERS,
                MAX_OFFLINE_RENDER_WORKERS,
                self.offline_render_workers
            );
        }
        Ok(())
    }
}

impl From<&Config> for CoreConfig {
    fn from(value: &Config) -> Self {
        Self {
            output_midi: value.output_midi.clone(),
            output_wav: value.output_wav.clone(),
            sample_rate: value.sample_rate,
            buffer_size: value.buffer_size,
            patch_path: None,
            patches_dir: crate::patches::core_config_patch_root_dir(value),
            random_patch: false,
        }
    }
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
