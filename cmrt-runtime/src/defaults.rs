use serde::Serialize;

// プラグインの標準インストール先と音色置き場は play server repo 側が単一ソース。
// ここ（config.toml のひな形生成）は TUI 固有なので、値だけを借りて組み立てる。
pub use cmrt_server_config::{
    default_dexed_cartridge_dirs, default_dexed_plugin_path, default_floe_plugin_path,
    default_patches_dirs, default_plugin_path, default_vaporizer2_plugin_path,
};

use cmrt_patches::surge_xt::{
    DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES, DEFAULT_BASS_PATCH_CATEGORY_NAMES,
    DEFAULT_CHORD_PATCH_CATEGORY_NAMES, DEFAULT_DRUM_PATCH_CATEGORY_NAMES,
    DEFAULT_HIHAT_PATCH_KEYWORDS, DEFAULT_KICK_PATCH_KEYWORDS, DEFAULT_SNARE_PATCH_KEYWORDS,
};
use cmrt_patches::vaporizer2::{
    DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES as VAPORIZER2_ARPEGGIO_CATEGORY_NAMES,
    DEFAULT_BASS_PATCH_CATEGORY_NAMES as VAPORIZER2_BASS_CATEGORY_NAMES,
    DEFAULT_CHORD_PATCH_CATEGORY_NAMES as VAPORIZER2_CHORD_CATEGORY_NAMES,
    DEFAULT_DRUM_PATCH_CATEGORY_NAMES as VAPORIZER2_DRUM_CATEGORY_NAMES,
    PATCH_CATEGORY_CODES as VAPORIZER2_PATCH_CATEGORY_CODES,
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
    let patch_roles_block = format!(
        "{}{}{}{}",
        surge_xt_patch_roles_block(),
        vaporizer2_patch_roles_block(),
        floe_profile_block(),
        other_plugin_profile_block()
    );
    format!(
        r#"# clap-mml-render-tui config
#
# 【必須】plugin_path にお使いの CLAP プラグインのパスを設定してください。
# 例 (Windows): plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'
# 例 (Linux):   plugin_path = '/usr/lib/clap/Surge XT.clap'
# 例 (macOS):   plugin_path = '/Library/Audio/Plug-Ins/CLAP/Surge XT.clap'
{plugin_path_line}

# 【省略可】複数プラグインを使い分ける場合は、active_plugin の1行で切り替えられます。
# 'Surge XT' / 'Dexed' / 'Vaporizer2' / 'Floe' は組み込みなので、標準の場所へインストールして
# あればプラグイン本体のパスは書かずに済みます（大文字小文字・空白・アンダースコアの
# 違いは無視されます）。active_plugin を書くと、上の plugin_path / patches_dirs は
# 使われません。
#
# active_plugin = 'Dexed'
#
# Vaporizer2 と Floe は音色置き場の既定値を持ちません（プリセットの置き場所が
# インストールごとに違うため）。末尾の各 [plugins.*] に patches_dirs を書いてください。
# 書かないプラグインの音色は一覧に出ません。
#
# 標準以外の場所に入れている場合や、組み込みに無いプラグインを使う場合だけ、
# [plugins.<名前>] を書きます。書いた項目だけが組み込みの値を上書きします。
# **書く場所はこのファイルの末尾**です（TOML はテーブル見出しから下がすべてその中身に
# なるので、途中に書くと後続のトップレベル項目が吸い込まれます）。
# 末尾のコメント済みブロックを参照してください。

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

{patch_roles_block}"#,
    )
}

/// 用途別 patch 自動選択の 7 項目を、**コメント済みの `[plugins."Surge XT"]` ブロック**として
/// 組み立てる。
///
/// トップレベルへ値として書き出さないのが要点。トップレベルの値は既定プラグインにだけ効く
/// レガシー綴りで、そこへ Surge のカテゴリ名を書き出すと `active_plugin = 'my_synth'` の
/// ような config で候補が全滅する（`docs/adr/0007-patch-role-defaults-three-layers.md`）。既定値は
/// [`crate::PatchRoles::builtin_for`] がプラグインごとに持つので、ここは**見て編集できる
/// ようにするための案内**でしかない。
///
/// **必ず config.toml の末尾へ置くこと。** TOML はテーブル見出しから下がすべてその
/// テーブルの中身になるので、途中に置くとコメントを外した瞬間に後続のトップレベル項目が
/// `[plugins."Surge XT"]` の中へ吸い込まれる。
fn surge_xt_patch_roles_block() -> String {
    let chord = patch_categories_line(&DEFAULT_CHORD_PATCH_CATEGORY_NAMES);
    let bass = patch_categories_line(&DEFAULT_BASS_PATCH_CATEGORY_NAMES);
    let arpeggio = patch_categories_line(&DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES);
    let drum = patch_categories_line(&DEFAULT_DRUM_PATCH_CATEGORY_NAMES);
    let kick = patch_categories_line(&DEFAULT_KICK_PATCH_KEYWORDS);
    let snare = patch_categories_line(&DEFAULT_SNARE_PATCH_KEYWORDS);
    let hihat = patch_categories_line(&DEFAULT_HIHAT_PATCH_KEYWORDS);
    format!(
        r#"# 【省略可】用途別 patch 自動選択のカテゴリ／キーワード（以下 7 項目）
# 音色置き場の体系ごとに正解が違うため、既定値はプラグインごとに組み込みで持っています。
# 下に書いてあるのが Surge XT の既定値です（そのまま効いているので、書かなくても
# 同じ結果になります）。cartridge を使う Dexed はカテゴリ階層を持たないので「絞らない」が
# 既定で、組み込みに無いプラグインも同じく「絞らない」が既定です。
#
# 変えたいときだけ、下の行のコメント（先頭の `# `）を外して書き換えてください。
# 【注意】ここから下はすべて [plugins."Surge XT"] の中身になります。トップレベルの項目を
# 足すときは、このブロックより上へ書いてください。
#
# [plugins."Surge XT"]
#
# 標準以外の場所に入れているときだけ書きます。書かなければ組み込みの値が使われます。
# plugin_path  = 'D:\my\clap\Surge XT.clap'
# patches_dirs = ['D:\my\patches']
#
# chord mode の和音に使う patch のカテゴリ。patch パスのカテゴリ階層
# （patches_factory/<category>/ または patches_3rdparty/<vendor>/<category>/）と、
# 大文字小文字を無視して照合します。ここから、さらに poly と判明している patch だけを
# 抽選します。空リストにするとカテゴリで絞らず、poly 判定だけで抽選します。
# chord_patch_categories = {chord}
#
# chord mode の bass 行（行2）に使う patch のカテゴリ。照合の仕方は同じです。
# bass は単音なので mono / poly は問いません。
# bass_patch_categories = {bass}
#
# chord mode のアルペジオ行（行3。4 voice の行）に使う patch のカテゴリ。
# 音程が意味を持つ行なので、既定では打楽器・効果音のカテゴリを外しています。
# arpeggio_patch_categories = {arpeggio}
#
# drum 行（track 数 4 以上のとき、行4以降）に使う patch のカテゴリ。4 役で共通です。
# Surge のカテゴリは打楽器を Percussion / Drums の粒度でしか分けていないため、
# kick / snare / hi-hat / percussion の振り分けは下のキーワードで行います。
# drum_patch_categories = {drum}
#
# drum 4 役に振り分けるための patch 名キーワード。上のカテゴリで絞ったあと、
# 小文字化した patch のパスへ部分一致させます。percussion 行には
# 「kick / snare / hi-hat のどれにも当たらなかったもの」が回ります。
# kick_patch_keywords = {kick}
# snare_patch_keywords = {snare}
# hihat_patch_keywords = {hihat}
"#
    )
}

/// Vaporizer2 の音色置き場と用途別カテゴリを、**コメント済みの `[plugins.Vaporizer2]`
/// ブロック**として組み立てる。
///
/// `patches_dirs` を書く欄があることが要点。Vaporizer2 は
/// [`cmrt_server_config::default_vaporizer2_plugin_path`] は持つが**音色置き場の既定値を
/// 持たない**（プリセットの置き場所が `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml` や
/// レジストリで決まるインストールごとの値なので、こちらから決め打ちできない）。
/// この 1 行が無いと `catalog_plugins` が Vaporizer2 を飛ばし、`.vvp` の音色は
/// 1 件も一覧に出ない。
///
/// カテゴリコードの対応表も併記する。`.vvp` のカテゴリは**ファイル名先頭 2 文字**で、
/// 生のコード（`PD`）のままでは下の用途別項目に書いた展開名（`Pad`）と照合できない。
/// 表が無いと、ユーザーは自分の音色置き場を見てもどう書けばよいか分からない。
fn vaporizer2_patch_roles_block() -> String {
    let chord = patch_categories_line(&VAPORIZER2_CHORD_CATEGORY_NAMES);
    let bass = patch_categories_line(&VAPORIZER2_BASS_CATEGORY_NAMES);
    let arpeggio = patch_categories_line(&VAPORIZER2_ARPEGGIO_CATEGORY_NAMES);
    let drum = patch_categories_line(&VAPORIZER2_DRUM_CATEGORY_NAMES);
    let category_codes = vaporizer2_category_code_lines();
    format!(
        r#"
# 【省略可】Vaporizer2（VAST Dynamics）の音色置き場と用途別カテゴリ。
# Vaporizer2 を使うときは patches_dirs の 1 行だけ必須です（プリセットの置き場所は
# インストールごとに違うので、こちらでは決め打ちできません）。プラグイン本体を標準の
# 場所へ入れてあれば plugin_path は要りません。
# 【注意】ここから下はすべて [plugins.Vaporizer2] の中身になります。
#
# [plugins.Vaporizer2]
# patches_dirs = ['D:\my\Vaporizer2\Presets']
# plugin_path  = 'D:\my\clap\VASTvaporizer2.clap'
#
# 用途別カテゴリは Vaporizer2 の体系で書きます。カテゴリは .vvp の**ファイル名先頭
# 2 文字**（'AR Accent Arp.vvp' なら AR）で、下の対応表の**展開名**のほうを書きます。
# Surge XT のカテゴリ名（複数形の Pads / Basses など）とは綴りが違うので流用できません。
{category_codes}
#
# chord_patch_categories = {chord}
# bass_patch_categories = {bass}
# arpeggio_patch_categories = {arpeggio}
#
# 出荷プリセットに Drum は 9 件しかないので、drum 4 役はほぼ空になります。
# drum_patch_categories = {drum}
#
# kick / snare / hi-hat の振り分けキーワードは Surge XT と同じ既定です
# （太鼓の一般名なのでプラグインに依りません）。
"#
    )
}

/// カテゴリコード → 展開名の対応表を、config.toml のコメント行へ折り返して並べる。
///
/// 表は [`cmrt_patches::vaporizer2`] が単一ソース。ここへ書き写すと、コードを足したときに
/// ひな形だけ古いまま残る。
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

/// Floe の preset root を指定するためのコメント済みプロファイル。
fn floe_profile_block() -> String {
    r#"
# 【省略可】Floe の音色置き場。Floe を使うときは patches_dirs を指定してください。
# プラグイン本体を標準の場所へ入れてあれば plugin_path / plugin_id は不要です。
# `.floe-preset` の先頭ディレクトリがカテゴリになり、用途別には絞り込みません。
#
# [plugins.Floe]
# patches_dirs = ['D:\my\Floe\presets']
# plugin_path  = 'D:\my\clap\Floe.clap'
"#
    .to_string()
}

/// 組み込みに無いプラグインを足すときの雛形。**ひな形の最後**に置く。
fn other_plugin_profile_block() -> String {
    r#"
# 組み込みに無いプラグインを足すときも、この下へ続けます。用途別 7 項目を書かなければ
# 「絞らない」が既定なので、まずは plugin_path と patches_dirs だけで動きます。
#
# [plugins.my_synth]
# plugin_path  = 'D:\my\clap\MySynth.clap'
# patches_dirs = ['D:\my\patches']
"#
    .to_string()
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
