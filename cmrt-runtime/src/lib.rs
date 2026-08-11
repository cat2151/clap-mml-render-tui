mod defaults;
mod paths;

pub use defaults::{
    default_config_content, default_config_content_with_app_settings, default_patches_dirs,
    default_plugin_path, serialize_patches_dirs_line,
};
pub use paths::{config_app_dir, config_file_path, log_file_path, native_probe_log_file_path};

use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// app の HTTP サーバー（`--server` CLI モード / DAW HTTP サーバー）が listen する localhost port。
pub const DEFAULT_PORT: u16 = 62151;
pub const DEFAULT_OFFLINE_RENDER_WORKERS: usize = 2;
pub const DEFAULT_OFFLINE_RENDER_SERVER_WORKERS: usize = 4;
pub const DEFAULT_OFFLINE_RENDER_SERVER_PORT: u16 = 62153;
pub const DEFAULT_REALTIME_PLAY_SERVER_PORT: u16 = 62154;
pub const DEFAULT_VOICING_SHARED_SOURCE: &str =
    "https://raw.githubusercontent.com/cat2151/cat-music-patterns/main/surge-xt-patch-voicing.json";
pub const DEFAULT_VOICING_OVERRIDE_SOURCE: &str = "https://raw.githubusercontent.com/cat2151/cat-music-patterns/main/surge-xt-patch-voicing-overrides.json";
pub const DEFAULT_CHORD_PROGRESSION_SOURCE: &str =
    "https://raw.githubusercontent.com/cat2151/cat-music-patterns/main/chord-progressions.json";
pub const DEFAULT_LOOP_CATEGORY_NAMES: [&str; 5] = ["guitar", "drum", "bass", "spoken", "sequence"];
const MIN_OFFLINE_RENDER_WORKERS: usize = 1;
const MAX_OFFLINE_RENDER_WORKERS: usize = 16;

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
    /// WAV ループブラウザーの検索対象ディレクトリ一覧
    #[serde(default)]
    pub loop_dirs: Vec<String>,
    /// WAV ループディレクトリへ付与できるカテゴリ一覧
    #[serde(default = "default_loop_categories")]
    pub loop_categories: Vec<String>,
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
    /// app 起動直後に realtime play server を先行起動するかどうか。
    #[serde(default = "default_realtime_play_server_prewarm")]
    pub realtime_play_server_prewarm: bool,
    /// 起動時に自動再生するかどうか（notepad: 現在行 / DAW: 曲先頭から演奏開始）
    #[serde(default = "default_autoplay_on_startup")]
    pub autoplay_on_startup: bool,
    /// keyboard の共有 mono/poly 判定JSON。HTTP(S) URLまたはconfig.toml基準のpath。
    #[serde(default = "default_voicing_shared_source")]
    pub voicing_shared_source: String,
    /// keyboard の mono/poly 判定override JSON。HTTP(S) URLまたはconfig.toml基準のpath。
    #[serde(default = "default_voicing_override_source")]
    pub voicing_override_source: String,
    /// grid sequencer の chord mode が使うコード進行JSON。HTTP(S) URLまたはconfig.toml基準のpath。
    #[serde(default = "default_chord_progression_source")]
    pub chord_progression_source: String,
    /// chord mode の和音に使う patch のカテゴリ一覧。空にすると全カテゴリが対象。
    #[serde(default = "default_chord_patch_categories")]
    pub chord_patch_categories: Vec<String>,
    /// chord mode の bass 行に使う patch のカテゴリ一覧。空にすると全カテゴリが対象。
    #[serde(default = "default_bass_patch_categories")]
    pub bass_patch_categories: Vec<String>,
    /// chord mode のアルペジオ行（4 voice の行）に使う patch のカテゴリ一覧。
    /// 空にすると全カテゴリが対象。
    #[serde(default = "default_arpeggio_patch_categories")]
    pub arpeggio_patch_categories: Vec<String>,
    /// drum 行に使う patch のカテゴリ一覧。4 役（kick / snare / hi-hat / percussion）で共通。
    /// 空にすると全カテゴリが対象。
    #[serde(default = "default_drum_patch_categories")]
    pub drum_patch_categories: Vec<String>,
    /// kick 行に使う patch の名前キーワード一覧。上のカテゴリの中をさらに絞る。
    /// 空にするとカテゴリだけで絞る。
    #[serde(default = "default_kick_patch_keywords")]
    pub kick_patch_keywords: Vec<String>,
    /// snare 行に使う patch の名前キーワード一覧。
    #[serde(default = "default_snare_patch_keywords")]
    pub snare_patch_keywords: Vec<String>,
    /// hi-hat 行に使う patch の名前キーワード一覧。
    #[serde(default = "default_hihat_patch_keywords")]
    pub hihat_patch_keywords: Vec<String>,
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

fn default_realtime_play_server_prewarm() -> bool {
    true
}

fn default_autoplay_on_startup() -> bool {
    true
}

pub fn default_loop_categories() -> Vec<String> {
    DEFAULT_LOOP_CATEGORY_NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

fn default_voicing_shared_source() -> String {
    DEFAULT_VOICING_SHARED_SOURCE.to_string()
}

fn default_voicing_override_source() -> String {
    DEFAULT_VOICING_OVERRIDE_SOURCE.to_string()
}

fn default_chord_progression_source() -> String {
    DEFAULT_CHORD_PROGRESSION_SOURCE.to_string()
}

pub fn default_chord_patch_categories() -> Vec<String> {
    to_category_names(&cmrt_surge_patches::DEFAULT_CHORD_PATCH_CATEGORY_NAMES)
}

pub fn default_bass_patch_categories() -> Vec<String> {
    to_category_names(&cmrt_surge_patches::DEFAULT_BASS_PATCH_CATEGORY_NAMES)
}

pub fn default_arpeggio_patch_categories() -> Vec<String> {
    to_category_names(&cmrt_surge_patches::DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES)
}

pub fn default_drum_patch_categories() -> Vec<String> {
    to_category_names(&cmrt_surge_patches::DEFAULT_DRUM_PATCH_CATEGORY_NAMES)
}

pub fn default_kick_patch_keywords() -> Vec<String> {
    to_category_names(&cmrt_surge_patches::DEFAULT_KICK_PATCH_KEYWORDS)
}

pub fn default_snare_patch_keywords() -> Vec<String> {
    to_category_names(&cmrt_surge_patches::DEFAULT_SNARE_PATCH_KEYWORDS)
}

pub fn default_hihat_patch_keywords() -> Vec<String> {
    to_category_names(&cmrt_surge_patches::DEFAULT_HIHAT_PATCH_KEYWORDS)
}

fn to_category_names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
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
        if self.loop_dirs.iter().any(|dir| dir.trim().is_empty()) {
            anyhow::bail!("loop_dirs に空のディレクトリは指定できません");
        }
        if self.loop_categories.len() > 26 {
            anyhow::bail!("loop_categories は26件以下で指定してください");
        }
        if self
            .loop_categories
            .iter()
            .any(|category| category.trim().is_empty())
        {
            anyhow::bail!("loop_categories に空のカテゴリは指定できません");
        }
        let mut categories = HashSet::new();
        if self
            .loop_categories
            .iter()
            .any(|category| !categories.insert(category))
        {
            anyhow::bail!("loop_categories に重複したカテゴリは指定できません");
        }
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

/// アプリ設定からレンダリング用の `CoreConfig` を組み立てる。
/// notepad / DAW / offline render / server の各経路で共有する。
pub fn core_config_from_config(cfg: &Config) -> cmrt_core::CoreConfig {
    cmrt_core::CoreConfig {
        output_midi: cfg.output_midi.clone(),
        output_wav: cfg.output_wav.clone(),
        sample_rate: cfg.sample_rate,
        buffer_size: cfg.buffer_size,
        patch_path: None,
        patches_dir: core_config_patch_root_dir(cfg),
        random_patch: false,
    }
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
mod tests;
