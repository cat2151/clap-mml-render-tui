use serde::Serialize;

use crate::{
    DEFAULT_OFFLINE_RENDER_SERVER_PORT, DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
    DEFAULT_OFFLINE_RENDER_WORKERS, DEFAULT_REALTIME_PLAY_SERVER_PORT,
    DEFAULT_VOICING_OVERRIDE_SOURCE, DEFAULT_VOICING_SHARED_SOURCE,
};

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
offline_render_workers = {DEFAULT_OFFLINE_RENDER_WORKERS}

# 【省略可】オフラインレンダリング backend
# in_process: 従来どおり cmrt 本体プロセス内でレンダリングします。
# render_server: 127.0.0.1 の render-server 子プロセスへ POST /render します。
offline_render_backend = "in_process"
offline_render_server_workers = {DEFAULT_OFFLINE_RENDER_SERVER_WORKERS}
offline_render_server_port = {DEFAULT_OFFLINE_RENDER_SERVER_PORT}
offline_render_server_command = ""

# 【省略可】リアルタイム再生 backend
# in_process: 従来どおり cmrt 本体プロセス内で再生します。
# play_server: 127.0.0.1 の realtime play server 子プロセスへ POST /play します。
realtime_audio_backend = "in_process"
realtime_play_server_port = {DEFAULT_REALTIME_PLAY_SERVER_PORT}
realtime_play_server_command = ""

# 【省略可】app 起動直後に realtime play server を先行起動するかどうか
# keyboard / grid sequencer は CLAP インスタンスを最大16個作るサーバーを使い、その起動に
# 数秒かかります。true にすると app 起動直後にバックグラウンドで起動を済ませるため、
# 画面へ入ったときの待ち時間が音色ロードだけになります。
# false にすると画面へ入った時点で起動を始めます（メモリとCPUを常時使いたくない場合）。
realtime_play_server_prewarm = true

# 【省略可】起動時に自動再生するかどうか
# notepad モード: 現在行を即座に再生します。DAW モード: 曲先頭（measure 0）から演奏開始します。
# false にすると、起動直後は再生されず、Enter/Space（notepad）・Shift+P（DAW）などの
# キー操作で再生します。
autoplay_on_startup = true

# 【省略可】keyboard の patch mono/poly 判定データ
# HTTP(S) URL、絶対path、またはこのconfig.tomlからの相対pathを指定できます。空文字で無効化します。
voicing_shared_source = "{DEFAULT_VOICING_SHARED_SOURCE}"
voicing_override_source = "{DEFAULT_VOICING_OVERRIDE_SOURCE}"

# 【省略可】Surge XT パッチの検索対象ディレクトリ一覧（TUI / DAW の音色選択・ランダム音色で使う）
# 例 (Windows): patches_dirs = ['C:\ProgramData\Surge XT\patches_factory', 'C:\ProgramData\Surge XT\patches_3rdparty']
# 例 (Linux):   patches_dirs = ['/home/user/.local/share/surge-data/patches_factory', '/home/user/.local/share/surge-data/patches_3rdparty']
# 例 (macOS):   patches_dirs = ['/Library/Application Support/Surge XT/patches_factory', '/Library/Application Support/Surge XT/patches_3rdparty']
{patches_dirs_line}

# 【省略可】WAV ループブラウザーの検索対象ディレクトリ一覧
# 設定後に `cmrt scan-loops` を実行してインデックスを作成してください。
loop_dirs = []

# 【省略可】WAV ループディレクトリへ付与できるカテゴリ一覧
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

"#,
    )
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
