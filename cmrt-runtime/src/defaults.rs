use serde::Serialize;

// プラグインの標準インストール先と音色置き場はplay server repo側が単一ソース。
pub use cmrt_server_config::{
    default_dexed_cartridge_dirs, default_dexed_plugin_path, default_floe_plugin_path,
    default_patches_dirs, default_plugin_path, default_sforzando_plugin_path,
    default_vaporizer2_plugin_path,
};

use cmrt_server_config::VAPORIZER2_CATEGORY_CODES as VAPORIZER2_PATCH_CATEGORY_CODES;

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

/// OSに応じたデフォルトのconfig.toml内容を生成する。
pub fn default_config_content() -> String {
    default_config_content_with_app_settings("")
}

/// app側の追加設定を含めたデフォルトのconfig.toml内容を生成する。
pub fn default_config_content_with_app_settings(app_settings: &str) -> String {
    let app_settings = if app_settings.trim().is_empty() {
        String::new()
    } else {
        format!("{}\n", app_settings.trim_end())
    };
    let profile_blocks = format!(
        "{}{}{}{}{}",
        surge_xt_profile_block(),
        vaporizer2_profile_block(),
        floe_profile_block(),
        sforzando_profile_block(),
        other_plugin_profile_block()
    );
    format!(
        r#"# clap-mml-render-tui config
#
# 以下の plugin_path 例は、標準外の場所を使う場合に末尾の [plugins."Surge XT"] 内へ書きます。
# 例 (Windows): plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'
# 例 (Linux):   plugin_path = '/usr/lib/clap/Surge XT.clap'
# 例 (macOS):   plugin_path = '/Library/Audio/Plug-Ins/CLAP/Surge XT.clap'
# 既定プラグインは Surge XT 固定です。Surge XT / Dexed / Vaporizer2 / Floe / Sforzando は
# 組み込みなので、標準の場所へインストールしてあれば plugin_path は記述不要です。
# 標準値を変更するときだけ、ファイル末尾の [plugins.<名前>] に差分を書きます。
#
# Vaporizer2 と Floe は音色置き場の既定値を持ちません。末尾の各 [plugins.*] に
# patches_dirs を書いてください。書かないプラグインの音色は一覧に出ません。
#
# 標準以外の場所に入れている場合や、組み込みに無いプラグインを使う場合だけ、
# [plugins.<名前>] を書きます。書いた項目だけが組み込みの値を上書きします。
# **書く場所はこのファイルの末尾**です。

{app_settings}
input_midi  = "input.mid"
# output_midi, output_wav は自動的にシステム設定ディレクトリの clap-mml-render-tui/phrase/ または clap-mml-render-tui/daw/ に保存されます。
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512

# 【省略可】オフラインレンダリング同時実行数（1〜16）
offline_render_workers = {DEFAULT_OFFLINE_RENDER_WORKERS}

# 【省略可】オフラインレンダリング backend
offline_render_backend = "in_process"
offline_render_server_workers = {DEFAULT_OFFLINE_RENDER_SERVER_WORKERS}
offline_render_server_port = {DEFAULT_OFFLINE_RENDER_SERVER_PORT}
offline_render_server_command = ""

# 【省略可】リアルタイム再生 backend
realtime_audio_backend = "in_process"
realtime_play_server_port = {DEFAULT_REALTIME_PLAY_SERVER_PORT}
realtime_play_server_command = ""

# 【省略可】app 起動直後に realtime play server を先行起動するかどうか
realtime_play_server_prewarm = true

# 【省略可】起動時に自動再生するかどうか
autoplay_on_startup = true

# 【省略可】keyboard の patch mono/poly 判定データ
voicing_shared_source = "{DEFAULT_VOICING_SHARED_SOURCE}"
voicing_override_source = "{DEFAULT_VOICING_OVERRIDE_SOURCE}"

# 【省略可】grid sequencer の chord mode が使うコード進行データ
chord_progression_source = "{DEFAULT_CHORD_PROGRESSION_SOURCE}"

# Surge XT の標準音色ディレクトリを変える場合は末尾の profile に patches_dirs を書きます。
# 例 (Windows): patches_dirs = ['C:\ProgramData\Surge XT\patches_factory', 'C:\ProgramData\Surge XT\patches_3rdparty']
# 例 (Linux):   patches_dirs = ['/home/user/.local/share/surge-data/patches_factory', '/home/user/.local/share/surge-data/patches_3rdparty']
# 例 (macOS):   patches_dirs = ['/Library/Application Support/Surge XT/patches_factory', '/Library/Application Support/Surge XT/patches_3rdparty']

# 【省略可】WAV ループブラウザーの検索対象ディレクトリ一覧
loop_dirs = []

# 【省略可】WAV ループディレクトリへ付与できるカテゴリ一覧
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

{profile_blocks}"#,
    )
}

fn surge_xt_profile_block() -> String {
    r#"# 【省略可】Surge XTの標準値を変える場合だけコメントを外します。
#
# [plugins."Surge XT"]
# plugin_path  = 'D:\my\clap\Surge XT.clap'
# patches_dirs = ['D:\my\patches']
"#
    .to_string()
}

/// Vaporizer2の音色置き場と、selectorに表示するカテゴリコード表。
fn vaporizer2_profile_block() -> String {
    let category_codes = vaporizer2_category_code_lines();
    format!(
        r#"
# 【省略可】Vaporizer2（VAST Dynamics）の音色置き場。
# Vaporizer2を使うときはpatches_dirsを指定してください。
#
# [plugins.Vaporizer2]
# patches_dirs = ['D:\my\Vaporizer2\Presets']
# plugin_path  = 'D:\my\clap\VASTvaporizer2.clap'
#
# .vvpファイル名先頭2文字をselectorのCategory列では次の名前へ展開します。
{category_codes}
"#
    )
}

fn vaporizer2_category_code_lines() -> String {
    const PER_LINE: usize = 4;
    VAPORIZER2_PATCH_CATEGORY_CODES
        .chunks(PER_LINE)
        .map(|chunk| {
            let pairs = chunk
                .iter()
                .map(|(code, name)| format!("{code} {name}"))
                .collect::<Vec<_>>()
                .join(" / ");
            format!("#   {pairs}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn floe_profile_block() -> String {
    r#"
# 【省略可】Floeの音色置き場。Floeを使うときはpatches_dirsを指定してください。
# `.floe-preset` の先頭ディレクトリがselectorのCategory列になります。
#
# [plugins.Floe]
# patches_dirs = ['D:\my\Floe\presets']
# plugin_path  = 'D:\my\clap\Floe.clap'
"#
    .to_string()
}

fn sforzando_profile_block() -> String {
    r#"
# 【省略可】sforzandoのSFZ音色置き場。
#
# [plugins.Sforzando]
# patches_dirs = ['D:\my\sfz']
# plugin_path  = 'D:\my\clap\sforzando_x64.clap'
"#
    .to_string()
}

fn other_plugin_profile_block() -> String {
    r#"
# 組み込みに無いプラグインを足す場合。
#
# [plugins.my_synth]
# plugin_path  = 'D:\my\clap\MySynth.clap'
# patches_dirs = ['D:\my\patches']
"#
    .to_string()
}

/// patches_dirsの1行を安全なTOML文字列として生成する。
pub fn serialize_patches_dirs_line(patches_dirs: &[String]) -> String {
    toml::to_string(&PatchesDirsToml { patches_dirs })
        .unwrap_or_else(|_| "patches_dirs = []".to_string())
        .trim()
        .to_string()
}
