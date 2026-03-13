use serde::Deserialize;
use std::path::PathBuf;

const DEFAULT_CONFIG: &str = include_str!("../config.toml");

#[derive(Deserialize, Debug)]
pub struct Config {
    pub plugin_path: String,
    #[allow(dead_code)]
    pub input_midi: String,
    pub output_midi: String,
    pub output_wav: String,
    pub sample_rate: f64,
    pub buffer_size: usize,
    /// オプション: .fxp パッチファイルのパス。指定しない場合は Init Saw のまま。
    pub patch_path: Option<String>,
    /// ファクトリパッチのルートディレクトリ
    pub patches_dir: Option<String>,
    /// true のとき演奏ごとにランダムなパッチを選ぶ（デフォルト true）
    #[serde(default = "default_random_patch")]
    pub random_patch: bool,
}

fn default_random_patch() -> bool { true }

/// OS 標準の設定ディレクトリ内の config.toml パスを返す。
/// - Windows: %APPDATA%\cmrt\config.toml
/// - Linux:   ~/.config/cmrt/config.toml
/// - macOS:   ~/Library/Application Support/cmrt/config.toml
/// システムの設定ディレクトリが取得できない場合は `None` を返す。
pub fn config_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("cmrt").join("config.toml"))
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = config_file_path()
            .ok_or_else(|| anyhow::anyhow!("システムの設定ディレクトリが取得できません。HOME 環境変数などを確認してください。"))?;
        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, DEFAULT_CONFIG)
                .map_err(|e| anyhow::anyhow!("デフォルト config.toml の書き込みに失敗: {}", e))?;
            println!("デフォルトの config.toml を作成しました: {}", path.display());
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("config.toml が読めない: {}", e))?;
        toml::from_str(&text).map_err(|e| anyhow::anyhow!("config.toml のパースに失敗: {}", e))
    }
}
