use serde::Serialize;

use cmrt_surge_patches::{
    DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES, DEFAULT_BASS_PATCH_CATEGORY_NAMES,
    DEFAULT_CHORD_PATCH_CATEGORY_NAMES, DEFAULT_DRUM_PATCH_CATEGORY_NAMES,
    DEFAULT_HIHAT_PATCH_KEYWORDS, DEFAULT_KICK_PATCH_KEYWORDS, DEFAULT_SNARE_PATCH_KEYWORDS,
};

use crate::{
    DEFAULT_CHORD_PROGRESSION_SOURCE, DEFAULT_OFFLINE_RENDER_SERVER_PORT,
    DEFAULT_OFFLINE_RENDER_SERVER_WORKERS, DEFAULT_OFFLINE_RENDER_WORKERS,
    DEFAULT_REALTIME_PLAY_SERVER_PORT, DEFAULT_VOICING_OVERRIDE_SOURCE,
    DEFAULT_VOICING_SHARED_SOURCE,
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

/// OS ごとのデフォルト Dexed パスを返す。
/// `active_plugin = 'Dexed'` の 1 行だけで使えるようにするための組み込み値。
/// 既知 OS でない場合は空文字を返す（ユーザーに設定を促す）。
#[cfg(target_os = "windows")]
pub fn default_dexed_plugin_path() -> &'static str {
    r"C:\Program Files\Common Files\CLAP\Dexed.clap"
}

#[cfg(target_os = "macos")]
pub fn default_dexed_plugin_path() -> &'static str {
    "/Library/Audio/Plug-Ins/CLAP/Dexed.clap"
}

#[cfg(target_os = "linux")]
pub fn default_dexed_plugin_path() -> &'static str {
    "/usr/lib/clap/Dexed.clap"
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn default_dexed_plugin_path() -> &'static str {
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

/// OS ごとのデフォルト Dexed cartridge ディレクトリを返す。
///
/// Dexed が初回起動時に factory cartridge を展開する場所。`.syx` 1 個が
/// 32 program に展開される（`cmrt_core::dx7`）。
/// 既知 OS でない場合や取得できない場合は空配列を返す（ユーザーに設定を促す）。
#[cfg(target_os = "windows")]
pub fn default_dexed_cartridge_dirs() -> Vec<String> {
    dirs::config_dir()
        .map(|dir| {
            vec![dir
                .join("DigitalSuburban")
                .join("Dexed")
                .join("Cartridges")
                .to_string_lossy()
                .into_owned()]
        })
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
pub fn default_dexed_cartridge_dirs() -> Vec<String> {
    dirs::data_dir()
        .map(|dir| {
            vec![dir
                .join("DigitalSuburban")
                .join("Dexed")
                .join("Cartridges")
                .to_string_lossy()
                .into_owned()]
        })
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
pub fn default_dexed_cartridge_dirs() -> Vec<String> {
    dirs::data_dir()
        .map(|dir| {
            vec![dir
                .join("DigitalSuburban")
                .join("Dexed")
                .join("Cartridges")
                .to_string_lossy()
                .into_owned()]
        })
        .unwrap_or_default()
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn default_dexed_cartridge_dirs() -> Vec<String> {
    Vec::new()
}

/// カテゴリ名・キーワードの配列を config.toml の TOML 配列リテラルへ直す。
fn patch_categories_line(names: &[&str]) -> String {
    format!(
        "[{}]",
        names
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(", ")
    )
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
    // 定数と config.toml のひな形がずれないよう、リテラルではなく定数から組み立てる。
    let chord_patch_categories_line = patch_categories_line(&DEFAULT_CHORD_PATCH_CATEGORY_NAMES);
    let bass_patch_categories_line = patch_categories_line(&DEFAULT_BASS_PATCH_CATEGORY_NAMES);
    let arpeggio_patch_categories_line =
        patch_categories_line(&DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES);
    let drum_patch_categories_line = patch_categories_line(&DEFAULT_DRUM_PATCH_CATEGORY_NAMES);
    let kick_patch_keywords_line = patch_categories_line(&DEFAULT_KICK_PATCH_KEYWORDS);
    let snare_patch_keywords_line = patch_categories_line(&DEFAULT_SNARE_PATCH_KEYWORDS);
    let hihat_patch_keywords_line = patch_categories_line(&DEFAULT_HIHAT_PATCH_KEYWORDS);
    format!(
        r#"# clap-mml-render-tui config
#
# 【必須】plugin_path にお使いの CLAP プラグインのパスを設定してください。
# 例 (Windows): plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'
# 例 (Linux):   plugin_path = '/usr/lib/clap/Surge XT.clap'
# 例 (macOS):   plugin_path = '/Library/Audio/Plug-Ins/CLAP/Surge XT.clap'
{plugin_path_line}

# 【省略可】複数プラグインを使い分ける場合は、active_plugin の1行で切り替えられます。
# 'Surge XT' と 'Dexed' は組み込みなので、標準の場所へインストールしてあれば
# この1行だけで済みます（大文字小文字・空白・アンダースコアの違いは無視されます）。
# active_plugin を書くと、上の plugin_path / patches_dirs は使われません。
#
# active_plugin = 'Dexed'
#
# 標準以外の場所に入れている場合や、組み込みに無いプラグインを使う場合だけ、
# 下のように書きます。書いた項目だけが組み込みの値を上書きします。
#
# [plugins."Surge XT"]
# plugin_path = 'D:\my\clap\Surge XT.clap'
#
# [plugins.my_synth]
# plugin_path  = 'D:\my\clap\MySynth.clap'
# patches_dirs = ['D:\my\patches']

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

# 【省略可】grid sequencer の chord mode が使うコード進行データ
# HTTP(S) URL、絶対path、またはこのconfig.tomlからの相対pathを指定できます。空文字で無効化します。
# 無効化すると chord mode（c キー）は使えません。
chord_progression_source = "{DEFAULT_CHORD_PROGRESSION_SOURCE}"

# 【省略可】chord mode の和音に使う patch のカテゴリ
# patch パスのカテゴリ階層（patches_factory/<category>/ または
# patches_3rdparty/<vendor>/<category>/）と、大文字小文字を無視して照合します。
# ここに挙げたカテゴリの中から、さらに poly と判明している patch だけを抽選します。
# 空リストにするとカテゴリで絞らず、poly 判定だけで抽選します。
chord_patch_categories = {chord_patch_categories_line}

# 【省略可】chord mode の bass 行（行2）に使う patch のカテゴリ
# 照合の仕方は chord_patch_categories と同じです。bass は単音なので mono / poly は問いません。
# 空リストにするとカテゴリで絞らず、全 patch から抽選します。
bass_patch_categories = {bass_patch_categories_line}

# 【省略可】chord mode のアルペジオ行（行3 = 4 voice の行）に使う patch のカテゴリ
# 照合の仕方は chord_patch_categories と同じです。mono / poly は問いません。
# 音程が意味を持つ行なので、既定では打楽器・効果音のカテゴリを外しています。
# 空リストにするとカテゴリで絞らず、全 patch から抽選します。
arpeggio_patch_categories = {arpeggio_patch_categories_line}

# 【省略可】drum 行（track 数 4 以上のとき、行4以降）に使う patch のカテゴリ
# 照合の仕方は chord_patch_categories と同じです。4 役で共通の設定です。
# Surge のカテゴリは打楽器を Percussion / Drums の粒度でしか分けていないため、
# kick / snare / hi-hat / percussion の振り分けは下のキーワードで行います。
drum_patch_categories = {drum_patch_categories_line}

# 【省略可】drum 4 役に振り分けるための patch 名キーワード
# 上の drum_patch_categories で絞ったあと、小文字化した patch のパスへ部分一致させます。
# percussion 行には「kick / snare / hi-hat のどれにも当たらなかったもの」が回ります。
# 空リストにするとキーワードで絞らず、カテゴリ内の全 patch が候補になります。
kick_patch_keywords = {kick_patch_keywords_line}
snare_patch_keywords = {snare_patch_keywords_line}
hihat_patch_keywords = {hihat_patch_keywords_line}

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
