# clap-mml-render-tui

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

### 設定

初回起動時に `config.toml` が自動作成されます。場所はOS標準の設定ディレクトリ配下です。

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

現在の設定例です。

```toml
# 【必須】使用する CLAP プラグイン
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

input_midi  = "input.mid"

# output_midi, output_wav は自動的に設定ディレクトリ配下の
# clap-mml-render-tui/phrase/ または clap-mml-render-tui/daw/ に保存されます。
# 以下の値は内部的に使用されます。
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# DAW のオフラインレンダリング同時実行数（1〜16）
offline_render_workers = 4

# オフラインレンダリング backend
# in_process: cmrt 本体プロセス内でレンダリングします。
# render_server: render-server 子プロセスへ POST /render してレンダリングします。
offline_render_backend = "in_process"
offline_render_server_port = 62153
offline_render_server_command = ""

# Surge XT パッチの検索対象ディレクトリ一覧
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]
```

設定項目は次のとおりです。

| 項目 | 既定値 | 説明 |
| --- | --- | --- |
| `plugin_path` | OSごとの Surge XT CLAP 標準パス | 使用する CLAP プラグインのパスです。空の場合は起動時にエラーになります。 |
| `input_midi` | `input.mid` | 内部処理用の入力MIDIファイル名です。 |
| `output_midi` | `output.mid` | 内部処理用の出力MIDIファイル名です。実際の保存先は設定ディレクトリ配下の `phrase/` または `daw/` です。 |
| `output_wav` | `output.wav` | 内部処理用の出力WAVファイル名です。実際の保存先は設定ディレクトリ配下の `phrase/` または `daw/` です。 |
| `sample_rate` | `48000` | レンダリング時のサンプルレートです。 |
| `buffer_size` | `512` | レンダリング時のバッファサイズです。 |
| `offline_render_workers` | `4` | DAW のオフラインレンダリング同時実行数です。`1`〜`16` の範囲で設定します。 |
| `offline_render_backend` | `in_process` | オフラインレンダリングの実行先です。`in_process` または `render_server` を指定します。 |
| `offline_render_server_port` | `62153` | `offline_render_backend = "render_server"` のときに使う `127.0.0.1` のポート番号です。`1`〜`65535` の範囲で設定します。 |
| `offline_render_server_command` | 空文字 | `render_server` backend の子プロセス起動コマンドです。空文字の場合は `cmrt` と同じディレクトリの `clap-mml-render-server` / `clap-mml-render-server.exe`、または `PATH` 上の同名コマンドを探します。 |
| `patches_dirs` | OSごとの Surge XT patches 標準ディレクトリ | TUI / DAW の音色選択・ランダム音色で検索するディレクトリ一覧です。未設定または空の場合、音色選択・ランダム音色は使えません。 |

OS別の `plugin_path` 既定値は次のとおりです。

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS別の `patches_dirs` 既定値は次のとおりです。

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty`（`XDG_DATA_HOME` 未設定時は `~/.local/share`）
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

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
