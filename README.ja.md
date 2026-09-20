# clap-mml-render-tui

### 概要
MML TUI DAW（のようなもの）。Surge XT / Dexed / Vaporizer2 / Floe / Sforzando のリッチな音をMMLで手軽に楽しめます。Rustで書かれています。

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

#### play serverの実体

音を鳴らすのは別プロセスの play server です。実体は次の順で決まり、最初に見つかったものを使います。

1. `--play-server <PATH>` で指定したフルパス（指定したものが無ければ、探索へ移らずエラーで止まります）
2. `cmrt` と同じディレクトリの `clap-mml-realtime-play-server`
3. 兄弟リポジトリの release ビルド（`../clap-mml-play-server/target/release/`）

PATHは見ません。debugビルドのサーバーは先読みが4〜5倍遅く、演奏が小節の頭で途切れます。
debugビルドや素性の分からない実体を掴んでいるときは、画面の右上に警告が出ます。

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### 対応オーディオプラグイン
- ※CLAP、Windows、アカウント登録なしで無料で入手できるもの、に絞っています
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando
- effect（DAW の track に直列で挿せます）
  - TONE3000
  - Surge XT Effects

### AI生成ドキュメント
- 以降、AIが追記した部分が読みづらいです。ときどきメンテしていきます

### keyboard画面

`v`キーで、keyboard画面へ移動します。

- `c d e f g a b`キー: ドレミファソラシを鳴らします

### chord chart画面

`Ctrl+G` → `C`キーで、chord chart画面へ移動します。

1曲ぶんのコード進行の「構成」を俯瞰・編集する画面です。カーソルを動かすと、その行のコード進行が鳴ります。
`h` `l`で行内のchordを1つずつたどると、指したchordだけが鳴ります。

- 左pane（Sections）: 素材となるコード進行を名前つきで定義します
- 右pane（Arrangement）: 定義したsectionの並びが曲になります。同じsectionを何度でも並べられます
- sectionの進行を直すと、曲中のその参照が全部まとめて変わります
- ヘッダの1行は、曲の頭に置くchord2mmlの指定（`Key=C BPM120` など）をそのまま出したものです
- 進行もヘッダもこの画面は解釈しません。打った文字列をそのまま持ちます
- 鳴るのはカーソル行のsection 1つだけです（曲全体は鳴りません）。進行編集画面からChord Chart全体の音色を選べます
- `h` `l`で行内のchordを1つずつ試聴できます。いま指しているchordは進行の中で反転して見えます
- ヘッダのKeyだけが試聴に渡ります（BPMは既定のままです）
- 試聴の転回とoctaveは常にauto voicingされます。Sectionsではsection内、Arrangementでは曲順全体のつながりから決まり、`h` `l`の単独試聴でも同じ転回形を保ちます

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ q:終了 ?:help                                                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

キーバインドです。画面下段の1行には`q`と`?`しか出しません。全量は`?`キーで画面内に出ます。

| キー | pane | 動作 |
|---|---|---|
| `Tab` | 共通 | pane移動（Sections ⇔ Arrangementのトグル） |
| `j` `k` `↓` `↑` | 共通 | カーソル移動（行。移った先の行のコード進行が全部鳴ります） |
| `h` `l` `←` `→` | 共通 | 行内のchord移動（指したchord 1つだけ鳴ります。行の端では隣の行へ繰り上がります） |
| `PgUp` `PgDn` | 共通 | カーソルを10行移動します |
| `dd` | 共通 | カーソル行を削除します（Sectionsで消すとArrangement上の参照も一緒に消えます） |
| `Alt+↑` `Alt+↓` | 共通 | カーソル行を上 / 下へ移動します |
| `b` | 共通 | ヘッダのKey / BPMを1行入力で書き換えます |
| `Shift+P` `Space` | 共通 | カーソル行のsectionを試聴します（鳴っていたら停止します） |
| `?` | 共通 | ヘルプ開閉（`Esc`でも閉じます） |
| `q` | 共通 | アプリを終了します |
| `g` | Sections | コード進行カタログから抽選してsectionを追加します |
| `r` | Sections | カーソル行の進行を、名前を保ったまま抽選し直します |
| `i` | Sections | 進行をChord Chart用の1行MML overlayで編集します |
| `n` | Sections | 名前を1行入力で編集します |
| `1`〜`9` | Arrangement | その番号のsectionをカーソルの次に挿入します |

Sectionsの`i`で開く編集画面は、現在の進行を初期値にしてカーソルを末尾へ置きます。
入力中はカーソル位置のchordが成立・変化するたびにauto voicingで鳴り、`Ctrl+Space`で進行全体を
同じvoicingで鳴らせます。読めない入力はMMLとして代替再生しませんが、そのまま保存できます。
`Enter`で前後の空白を除いた進行を確定・保存し、`Esc`では変更を破棄します。

編集画面内の`Ctrl+T`で、通常のMML overlayと同じ音色一覧を開けます。候補を移動しただけでは
音色は変わらず、`Enter`で確定、`Esc`で元の音色へ戻ります。Chord Chartの音色は画面全体で1つで、
通常の`Ctrl+P` MML overlayの音色とは別に`history.json`へ保存されます。音色を確定したあとに
進行編集を`Esc`で破棄しても、確定した音色は残ります。

編集するたびに自動保存されます。保存先は設定ディレクトリ配下の
`clap-mml-render-tui/history/chord_chart.json` です（Windowsなら
`%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`）。保存する曲は常に1つです。
保存ファイルが無いときや読めないときは、画面を最初に開いた時点で`g`と同じ抽選を1回だけ行って
section 1つから始めます（抽選できなければ空のままです）。

`g` / `r` の抽選に使うコード進行カタログはネットワークから取得します。キャッシュがまだ無い初回だけ
待たされます（待ち時間は log.txt の
`chord-chart: event=catalog-first-load elapsed_ms=...` に残ります）。取得できていないときは
画面下段に「コード進行データがありません」と出ます。

### DAW画面のeffect chain

DAW画面のNORMALモードで、演奏trackにカーソルを置いて`x`を押すと、そのtrackのEFFECT CHAIN overlayが開きます。
instrument（音色）の後段に、TONE3000 / Surge XT Effects のfactory presetを直列で何段でも挿せます。

| キー | 動作 |
|---|---|
| `x` | カーソルtrackのEFFECT CHAIN overlayを開きます（chord行・conductor行では無効） |
| `j` `k` | chainの段を移動します |
| `a` | 追加overlayを開きます。全effectの全factory presetが1列に並ぶので`j` `k`で選び、`Enter`で末尾へ追加、`ESC`で戻ります |
| `dd` | カーソルの段を削除します |
| `Enter` | init列のJSONへ書き戻して閉じます（そのtrackのcache WAVが再レンダリングされます） |
| `ESC` | 変更を捨てて閉じます |

- chainはinit列のJSONの`"effects after instrument"`（配列の順＝信号の順）に保存されます。init列を直接編集しても同じです
- effectはcache WAVに焼き込まれます（render-server側で掛かります）
- 各effectのpresetは組み込みの既定の置き場（`%ProgramData%\TONE3000\Presets`、`%ProgramData%\Surge XT\fx_presets`）から読みます。pluginが無ければ候補に出ません
- chainは音符と同じ長さだけ回すので、リバーブの尻尾はcellの末尾で切れます

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

# オフラインレンダリングは render-server 子プロセスで行います。
# 同時実行数（1〜16）・port・起動コマンド（空なら実体を探索）
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# リアルタイム再生 backend（"cache_player" / "play_server"）
realtime_audio_backend = "cache_player"
realtime_play_server_port = 42154

# 起動時に自動再生するかどうか
# notepad モード: 現在行を即座に再生します。DAW モード: 曲先頭（measure 0）から演奏開始します。
autoplay_on_startup = true

# WAV ループブラウザーの検索対象ディレクトリ一覧
loop_dirs = []

# WAV ループディレクトリへ付与できるカテゴリ一覧
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

# Surge XT の標準値を変える場合だけ書く
[plugins."Surge XT"]
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]
```

設定項目は次のとおりです。

| 項目 | 既定値 | 説明 |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | OSごとの Surge XT CLAP 標準パス | Surge XT を標準外の場所へ入れた場合のパスです。 |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | 左から順に試す editor 候補です。 |
| `input_midi` | `input.mid` | 内部処理用の入力MIDIファイル名です。 |
| `output_midi` | `output.mid` | 内部処理用の出力MIDIファイル名です。 |
| `output_wav` | `output.wav` | 内部処理用の出力WAVファイル名です。 |
| `sample_rate` | `48000` | レンダリング時のサンプルレートです。 |
| `buffer_size` | `512` | レンダリング時のバッファサイズです。 |
| `offline_render_server_workers` | `4` | オフラインレンダリング（render-server）の同時実行数です。 |
| `offline_render_server_port` | `42153` | render-server の localhost port です。 |
| `offline_render_server_command` | 空文字 | render-server の起動コマンドです。空なら実体を探索します。 |
| `realtime_audio_backend` | `cache_player` | リアルタイム再生の実行先です（`cache_player` / `play_server`）。 |
| `realtime_play_server_port` | `42154` | play_server の localhost port です。 |
| `autoplay_on_startup` | `true` | 起動直後に自動再生するかどうかです。 |
| `plugins."Surge XT".patches_dirs` | OSごとの Surge XT patches 標準ディレクトリ | Surge XT の音色選択で検索するディレクトリ一覧です。 |
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

#### 固定の既定プラグインと複数プラグイン

音色を指定していない行が鳴る既定プラグインは **Surge XT 固定**です。`active_plugin` による切り替えはありません。Dexed など他のプラグインは `[plugins.<名前>]` から混在カタログへ追加され、音色を明示した行で使われます。

組み込みプロファイルの中身は次のとおりで、パスは OS ごとの標準インストール先です。

| 名前 | plugin_id | patches_dirs | 用途別カテゴリ |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | 上の表の OS 別既定値 | Surge XT のカテゴリ名 |
| `Dexed` | `com.digital-suburban.dexed` | Dexed の cartridge 置き場（Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`） | 全て空（＝絞らない） |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **既定値なし。`patches_dirs` を書いてください** | Vaporizer2 のカテゴリ名（`Pad` / `Bass` / `Arpeggio` など） |

名前は大文字小文字・空白・アンダースコアの違いを無視して照合します（`Dexed` / `dexed`、`Surge XT` / `surge_xt` / `SurgeXT` はすべて同じ）。

標準以外の場所に入れている場合、音色置き場を補う場合、または組み込みに無いプラグインを使う場合だけ `[plugins.<名前>]` を書きます。**書いた項目だけが組み込みの値を上書き**するので、Surge XT のパスを変えたいだけなら、その table 内の `plugin_path` 1行で足ります。

```toml
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
| `plugins.<名前>.plugin_path` | そのプラグインのパスです。 |
| `plugins.<名前>.plugin_id` | 期待する CLAP plugin ID です。省略できます。 |
| `plugins.<名前>.patches_dirs` | そのプラグインの音色置き場です。組み込みの値を消したいときは `patches_dirs = []` と書きます。 |
| `plugins.<名前>.<用途>_patch_categories` / `<役>_patch_keywords` | 用途別 patch 自動選択の絞り込みです。7 つのキー名（`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`）を書けます。書いた項目だけがそのプラグインのときに効きます。書かなければそのプラグインの既定値（Surge XT はカテゴリ名、それ以外は「絞らない」）が使われます。 |

- `active_plugin` は廃止済みです。また `plugin_path` / `plugin_id` / `patches_dirs` と用途別カテゴリ7項目をトップレベルへ書くと、黙って無視せず設定エラーにします。`active_plugin` は削除し、他の値は `[plugins."Surge XT"]` へ移してください。
- `[plugins.<名前>]` を追加しても既定プラグインは変わりません。Surge XT と同名の profile だけが固定既定値の override になり、他は混在カタログの候補になります。
- Dexed の音色は「cartridge の `.syx` 1個 = 32 program」なので、一覧では cartridge をディレクトリに見立てて `SynprezFM/SynprezFM_01.syx/00 Say Again.` のように 1 program ずつ並びます（番号は 0 始まりの2桁）。`patches_dirs` に cartridge の置き場を指定すれば、Surge の `.fxp` と同じように選べます。
- Dexed の mono/poly は音色ではなくインスタンスの設定（`MonoMode`）で、その既定値は POLY です。そのため grid sequencer の和音行では Dexed の音色をすべて和音向きとして扱います。
- Vaporizer2 の音色は `.vvp` ファイル1個 = 1音色で、Surge の `.fxp` と同じように選べます。一覧の見出しに出るカテゴリは**ファイル名の先頭2文字**（`AR Accent Arp.vvp` なら `AR` = `Arpeggio`）です。
- Vaporizer2 だけは `patches_dirs` の既定値を持ちません。プリセットの置き場がプラグイン側のグローバル設定（`%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml` など）で決まる環境依存の値で、そこを cmrt が勝手に読み書きするとお使いのDAW環境を壊すためです。次のように1行書いてください。書くまでは音色0件としてカタログに載りません。

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2 の mono/poly は音色ごとに違い、`.vvp` の中身（`m_uPolyMode`）から読みます。そのため grid sequencer の和音行には、和音の鳴る音色だけが候補として出ます（読めなかった音色は和音行の候補に出しません）。
- Vaporizer2 の出荷プリセットのうち、名前に `MPE` が付くものは cmrt では音が出ません。MPE（ノートごとのピッチ・プレッシャー）の演奏情報を前提にした音色で、cmrt はそれを送らないためです。
- 行の用途（chord / bass / arpeggio / drum）で候補を絞るカテゴリ設定の既定値は、**プラグインごとに違います**。Surge XT は Surge のカテゴリ名、Vaporizer2 は Vaporizer2 のカテゴリ名、Dexed と組み込みに無いプラグインは「絞らない」（＝どの行も全 program が候補）です。Dexed の cartridge は「ディレクトリ名＝用途」ではないためで、組み込みに無いプラグインも音色置き場の体系が分からないので絞りません。変えたいときは `[plugins.<名前>]` に 7 項目を書いてください（生成される config.toml の末尾に、Surge XT の既定値をコメントで載せてあります）。
- 用途別カテゴリ7項目も plugin profile の中だけに書きます。Surge XT 用なら `[plugins."Surge XT"]`、他のプラグイン用ならその plugin 自身の table に置きます。
- 用途別の自動選択に使う mono/poly の共有判定データ（`voicing_shared_source` / `voicing_override_source`）は Surge XT の音色判定にだけ使います。
- レンダリング結果のキャッシュはプラグインごとに別ディレクトリへ置くので、混在しても別プラグインの音を誤利用しません（手で消す必要はありません）。置き場は次の2つで、`<プラグイン>` は解決済み `plugin_path` のファイル名（拡張子なし）です（Windows の場合）。
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<プラグイン>\*.wav`（notepad / MML入力overlay のキャッシュ）
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<プラグイン>\*.wav`（DAW のトラックWAV）

オフラインレンダリングはすべてrender-server経由です。TUI側（`cmrt.exe`）はCLAPプラグインをロードせず、`127.0.0.1:<offline_render_server_port>/render` にMMLを送ってWAVを受け取ります。render-serverへの接続に失敗した場合、cmrtは子プロセスを起動し、通信エラー時は一度だけ再起動して再試行します。子プロセスの実体は`offline_render_server_command`が空なら「`cmrt.exe`と同じディレクトリ→兄弟repo `clap-mml-play-server`のreleaseビルド」の順で探し、**PATHは見ません**。

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
