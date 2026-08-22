# clap-mml-render-tui

### 概要
MML TUI DAW（のようなもの）。Surge XTのリッチな音をMMLで手軽に楽しめます。Rustで書かれています。

### 用途

- MMLで音を鳴らして遊ぶ用
- カジュアルにインストールする用。RustがあるだけでOK

### 技術スタック
- プラグインホスト用ライブラリ
  - https://github.com/prokopyl/clack

### 準備

[Surge XT](https://surge-synthesizer.github.io/)をinstallしてください

```
winget install "Surge XT"
```

### install

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

### 実行

```
cmrt
```

TUI画面でMML入力して遊べます

### keyboard画面

`v`キーで、keyboard画面へ移動します。

- `c d e f g a b`キー: ドレミファソラシを鳴らします

### 設定

初回起動時に `config.toml` が自動作成されます。場所はOS標準の設定ディレクトリ配下です。

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

TUI / DAW の NORMAL モードで `e` を押すと `config.toml` を editor で開きます。editor を閉じた後はアプリを再起動します。

現在の設定例です。

```toml
# 【必須】使用する CLAP プラグイン
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# config.toml を開く editor 候補（左から順に試す）
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav は自動的に設定ディレクトリ配下の
# clap-mml-render-tui/phrase/ または clap-mml-render-tui/daw/ に保存されます。
# 以下の値は内部的に使用されます。
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# DAW のオフラインレンダリング同時実行数（1〜16）
offline_render_workers = 2

# オフラインレンダリング backend
# in_process: cmrt 本体プロセス内でレンダリングします。
# render_server: render-server 子プロセスへ POST /render してレンダリングします。
offline_render_backend = "in_process"
offline_render_server_workers = 4
offline_render_server_port = 62153
offline_render_server_command = ""

# リアルタイム再生 backend
realtime_audio_backend = "in_process"
realtime_play_server_port = 62154
realtime_play_server_command = ""

# 起動時に自動再生するかどうか
# notepad モード: 現在行を即座に再生します。DAW モード: 曲先頭（measure 0）から演奏開始します。
autoplay_on_startup = true

# Surge XT パッチの検索対象ディレクトリ一覧
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]

# WAV ループブラウザーの検索対象ディレクトリ一覧
loop_dirs = []

# WAV ループディレクトリへ付与できるカテゴリ一覧
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]
```

設定項目は次のとおりです。

| 項目 | 既定値 | 説明 |
| --- | --- | --- |
| `plugin_path` | OSごとの Surge XT CLAP 標準パス | 使用する CLAP プラグインのパスです。 |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | 左から順に試す editor 候補です。 |
| `input_midi` | `input.mid` | 内部処理用の入力MIDIファイル名です。 |
| `output_midi` | `output.mid` | 内部処理用の出力MIDIファイル名です。 |
| `output_wav` | `output.wav` | 内部処理用の出力WAVファイル名です。 |
| `sample_rate` | `48000` | レンダリング時のサンプルレートです。 |
| `buffer_size` | `512` | レンダリング時のバッファサイズです。 |
| `offline_render_workers` | `2` | in_process のレンダリング同時実行数です。 |
| `offline_render_backend` | `in_process` | オフラインレンダリングの実行先です。 |
| `offline_render_server_workers` | `4` | render_server の同時実行数です。 |
| `offline_render_server_port` | `62153` | render_server の localhost port です。 |
| `offline_render_server_command` | 空文字 | render_server の起動コマンドです。 |
| `realtime_audio_backend` | `in_process` | リアルタイム再生の実行先です。 |
| `realtime_play_server_port` | `62154` | play_server の localhost port です。 |
| `realtime_play_server_command` | 空文字 | play_server の起動コマンドです。 |
| `autoplay_on_startup` | `true` | 起動直後に自動再生するかどうかです。 |
| `patches_dirs` | OSごとの Surge XT patches 標準ディレクトリ | 音色選択で検索するディレクトリ一覧です。 |
| `loop_dirs` | `[]` | WAV ループブラウザーで検索するディレクトリ一覧です。変更後は `cmrt scan-loops` を実行します。 |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | loop dirへ割り当てるカテゴリ一覧です。カテゴリoverlayのキーはカテゴリ名中の未使用英字から決まります。 |

OS別の `plugin_path` 既定値は次のとおりです。

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS別の `patches_dirs` 既定値は次のとおりです。

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty`（`XDG_DATA_HOME` 未設定時は `~/.local/share`）
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### 複数プラグインの使い分け

`active_plugin` の1行で切り替えられます。`Surge XT` と `Dexed` は**組み込み**なので、標準の場所へインストールしてあれば書くのはこの1行だけです（`Vaporizer2` も組み込みですが、音色置き場だけは書く必要があります。下記）。

```toml
active_plugin = 'Dexed'
```

組み込みプロファイルの中身は次のとおりで、パスは OS ごとの標準インストール先です。

| 名前 | plugin_id | patches_dirs | 用途別カテゴリ |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | 上の表の OS 別既定値 | トップレベルの設定（＝ Surge のカテゴリ名）をそのまま使う |
| `Dexed` | `com.digital-suburban.dexed` | Dexed の cartridge 置き場（Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`） | 全て空（＝絞らない） |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **既定値なし。`patches_dirs` を書いてください** | Vaporizer2 のカテゴリ名（`Pad` / `Bass` / `Arpeggio` など） |

名前は大文字小文字・空白・アンダースコアの違いを無視して照合します（`Dexed` / `dexed`、`Surge XT` / `surge_xt` / `SurgeXT` はすべて同じ）。

標準以外の場所に入れている場合や、組み込みに無いプラグインを使う場合だけ `[plugins.<名前>]` を書きます。**書いた項目だけが組み込みの値を上書き**するので、パスを変えたいだけなら `plugin_path` の1行で足ります。

```toml
active_plugin = 'Surge XT'

# パスだけ差し替える。plugin_id と patches_dirs は組み込みの値のまま。
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# 組み込みに無いプラグインは全部書く。
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| 項目 | 説明 |
| --- | --- |
| `active_plugin` | 使うプロファイルの名前です。組み込みの名前か `[plugins.*]` の名前を書きます。書かなければトップレベルの `plugin_path` / `patches_dirs` をそのまま使います。 |
| `plugins.<名前>.plugin_path` | そのプラグインのパスです。 |
| `plugins.<名前>.plugin_id` | 期待する CLAP plugin ID です。省略できます。 |
| `plugins.<名前>.patches_dirs` | そのプラグインの音色置き場です。組み込みの値を消したいときは `patches_dirs = []` と書きます。 |
| `plugins.<名前>.<用途>_patch_categories` / `<役>_patch_keywords` | 用途別 patch 自動選択の絞り込みです。7 つのキー名（`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`）を書けます。書いた項目だけがそのプラグインのときに効きます。書かなければそのプラグインの既定値（Surge XT はカテゴリ名、それ以外は「絞らない」）が使われます。 |

- `active_plugin` を書くと、トップレベルの `plugin_path` / `patches_dirs` は使われません（エラーにはならず、プロファイルが優先されます）。用途別カテゴリも、プロファイル側に書いてある項目（組み込み Dexed の 7 項目を含む）はプロファイルが優先されます。
- `active_plugin` の名前が組み込みにも `[plugins.*]` にも無い場合はエラーで起動しません。使える名前が両方とも表示されます。
- Dexed の音色は「cartridge の `.syx` 1個 = 32 program」なので、一覧では cartridge をディレクトリに見立てて `SynprezFM/SynprezFM_01.syx/00 Say Again.` のように 1 program ずつ並びます（番号は 0 始まりの2桁）。`patches_dirs` に cartridge の置き場を指定すれば、Surge の `.fxp` と同じように選べます。
- Dexed の mono/poly は音色ではなくインスタンスの設定（`MonoMode`）で、その既定値は POLY です。そのため grid sequencer の和音行では Dexed の音色をすべて和音向きとして扱います。
- Vaporizer2 の音色は `.vvp` ファイル1個 = 1音色で、Surge の `.fxp` と同じように選べます。一覧の見出しに出るカテゴリは**ファイル名の先頭2文字**（`AR Accent Arp.vvp` なら `AR` = `Arpeggio`）です。
- Vaporizer2 だけは `patches_dirs` の既定値を持ちません。プリセットの置き場がプラグイン側のグローバル設定（`%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml` など）で決まる環境依存の値で、そこを cmrt が勝手に読み書きするとお使いのDAW環境を壊すためです。次のように1行書いてください。書くまでは音色0件としてカタログに載りません。

```toml
active_plugin = 'Vaporizer2'

[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2 の mono/poly は音色ごとに違い、`.vvp` の中身（`m_uPolyMode`）から読みます。そのため grid sequencer の和音行には、和音の鳴る音色だけが候補として出ます（読めなかった音色は和音行の候補に出しません）。
- Vaporizer2 の出荷プリセットのうち、名前に `MPE` が付くものは cmrt では音が出ません。MPE（ノートごとのピッチ・プレッシャー）の演奏情報を前提にした音色で、cmrt はそれを送らないためです。
- 行の用途（chord / bass / arpeggio / drum）で候補を絞るカテゴリ設定の既定値は、**プラグインごとに違います**。Surge XT は Surge のカテゴリ名、Vaporizer2 は Vaporizer2 のカテゴリ名、Dexed と組み込みに無いプラグインは「絞らない」（＝どの行も全 program が候補）です。Dexed の cartridge は「ディレクトリ名＝用途」ではないためで、組み込みに無いプラグインも音色置き場の体系が分からないので絞りません。変えたいときは `[plugins.<名前>]` に 7 項目を書いてください（生成される config.toml の末尾に、Surge XT の既定値をコメントで載せてあります）。
- 7 項目はトップレベルにも書けますが、**効くのは既定プラグイン（音色を無指定にした行が鳴るもの）に対してだけ**です。`active_plugin` が無かった時代の書き方で、新しく生成する config.toml はトップレベルへ書きません。すでにトップレベルに書いてある config はそのまま動きます。
- 用途別の自動選択に使う mono/poly の共有判定データ（`voicing_shared_source` / `voicing_override_source`）は Surge XT 専用です。Surge XT 以外を使っているときは取得しません。
- レンダリング結果のキャッシュはプラグインごとに別ディレクトリへ置くので、切り替えても前のプラグインの音は鳴りません（手で消す必要はありません）。置き場は次の2つで、`<プラグイン>` は `plugin_path` のファイル名（拡張子なし）です（Windows の場合）。
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<プラグイン>\*.wav`（notepad / MML入力overlay のキャッシュ）
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw\<プラグイン>\*.wav`（DAW のトラックWAV）

`offline_render_backend = "render_server"` にすると、TUI側はCLAPプラグインを直接ロードせず、`127.0.0.1:<offline_render_server_port>/render` にMMLを送ってWAVを受け取ります。render-serverへの接続に失敗した場合、cmrtは子プロセスを起動し、通信エラー時は一度だけ再起動して再試行します。

### updateコマンド

```
cmrt update
```

### serverモード

```
cmrt --server
```

- bluesky-text-to-audio chrome拡張 と連動します
  - Blueskyの投稿にMMLがあったとき、それをSurge XTで鳴らせるようになります

### CLIモード

```
cmrt cde
```

- cdeと書けばドレミが鳴ります

```
cmrt CM7
```

- CM7と書けばCメジャーセブンスが鳴ります
- ほか、各種コード進行表記に対応しています（一部未対応のものがあります）

### patch-rolesコマンド

```
cmrt patch-roles
```

- grid sequencer の各行（chord / bass / アルペジオ / drum 4役 / それ以外）に、PATCH 欄の
  wheel で選べる音色の候補が何件あるかを表示します。画面は起動しません
- プラグインや `patches_dirs`、用途別カテゴリ（`chord_patch_categories` など）を変えたあと、
  「wheel を回しても無反応」になっていないかを確認するために使います
- 候補が0件の行があると、その行を挙げて終了コード 1 で終わります
- `--config <パス>` を付けると、その config.toml を読みます。設定を変えたら
  どうなるかを、いま使っている config.toml を書き換えずに試せます
- 複数プラグインの音色が並んでいるときは、用途ごとの候補数にプラグイン別の内訳も出ます。
  合計だけでは「あるプラグインの音色がその行へ1件も出ていない」ことに気づけないためです

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mmlコマンド

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- 指定した音色でMMLをオフラインレンダリングし、長さ・音量（`peak` / `rms`）・無音かどうか・
  出音のダイジェスト値を1行で表示します。画面は起動しません
- `patch-roles` が「音色が一覧に出るか」を数えるのに対し、こちらは「その音色が実際に音になるか」を見ます
- `--patch` は何個でも並べられます。まとめ行に「異なる出音 N / M」が出るので、
  **音色を替えたのに前の音のまま**になっていないかが分かります
- `--out-dir <ディレクトリ>` を付けるとWAVを書き出します（付けなければ1バイトも書きません）。
  耳で確かめたいときに使います
- `--poly-check` を付けると、和音と単音を鳴らし比べて、その音色が和音で鳴るかどうかを判定します
- `--config <パス>` は `patch-roles` と同じです

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# 破壊的変更
- 毎日頻繁に破壊的変更します

# 今後
- Surge XTのpatchesはAPIで取得するのが筋なのでそうする（今はtomlで指定したものを探索しており非効率。実装タイミングは後回し。ほかを優先している）

# コンセプトのメモ
- アトミック小節
    - Obsidianのアトミックノートに着想を得たものです。
    - あらゆる処理の単位を、「1小節単位のオフラインレンダリング」にすることで、
    - 制約を受けるかわりに、
    - いろいろなメリットを獲得できます。
    - これはスケッチ用途、素早く編集のサイクルをまわす用途に向きます。
    - より本格的な編集が必要なら、既存の高機能なDAWのほうが向くでしょう。
    - ※atomic measure だと物理学の言葉になってしまうので、ひとまず英訳せず「アトミック小節」のままにしておきます。

# スコープ外
- effectは編集必須なので割り切って、スコープ外、かなり後ろに後回し、とする。Surge XTの場合patchesがeffectsを内包している（effectsはpatchesから切り出したものである）、という点も理由の一つ
