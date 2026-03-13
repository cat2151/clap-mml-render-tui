use serde::Deserialize;

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

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let text = std::fs::read_to_string("config.toml")
            .map_err(|e| anyhow::anyhow!("config.toml が読めない: {}", e))?;
        toml::from_str(&text).map_err(|e| anyhow::anyhow!("config.toml のパースに失敗: {}", e))
    }
}
