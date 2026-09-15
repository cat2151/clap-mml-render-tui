# ADR 0011: 検証手段と実測ベースライン

- 状態: 採用
- 関連: すべての ADR

## `cmrt patch-roles` — 画面を開かずに wheel の候補を数える

grid sequencer の PATCH 欄の wheel が引く候補は「config の用途別カテゴリ」「patch 一覧」
「patch ごとの mono/poly 判定」の 3 つで決まる。プラグインを替えたときにどれか 1 つでも
噛み合わないと wheel が無反応になるが、それを見るには画面を開いて全行の wheel を
回すしかなかった。これを画面なしで数え、**候補 0 件の行があれば終了コード 1** で終わる。
使い方は README.ja.md の「patch-roles コマンド」。

**設計の肝**: `grid-sequencer/src/patch_role.rs` の `row_patch_purpose()` に行→用途の対応を
1 か所へ寄せ、**候補を数える述語を wheel と CLI で 1 本にした**。
ここを patch_selector 側へ戻すと「CLI が通っても画面が通らない」状態を作れてしまう。

`--config` は `Config::load_from_path()` を通るので、**既定の置き場を作りに行かない＝実ユーザーの
config.toml には 1 バイトも触らない**。設定を試すたびに実ファイルを書き換えて戻す運用は、
戻し忘れがそのまま本番の設定事故になる。
罠: `%LOCALAPPDATA%` を環境変数で差し替えても効かない。`dirs::config_local_dir()` は
Windows では Known Folder API を引くので env var は無視される。

drum 行の表は track 7 固定（track 4 は drum 行が 1 つで役割が抽選なので表に出せない）。

**カタログが 2 プラグイン以上のときは、用途ごとの候補数にプラグイン別の内訳も出す。**
合計だけでは「あるプラグインの音色がその役へ 1 件も出ていない」が他の数千件に埋もれて見えない。

- 引き分けは `PatchPlugins::index_for_patch()` ＝ **PATCH 欄の wheel と同じ述語**を通す
- **候補 0 件のプラグインも省略せず 0 と出す**（省略すると「0 件」と「数えていない」の
  区別が付かない）
- 「voicing 判定」欄にプラグインごとの `VoicingPolicy` が出る（[0008](0008-voicing-per-patch.md)）

**`[カタログから外したプラグイン]` 欄が、載らなかった理由を名指しで出す。**
上の内訳は載ったプラグインしか数えないので、「インストールしてあるのに 1 件も出ない」は
内訳を見ても分からない（行そのものが無い）。この欄がその 1 つ手前を見せる。

- **外れたものが無いときも「なし」と 1 行出す**（「無い」と「数えていない」を区別するため）
- 文言は `SkippedCatalogPlugin::notice_line()` が単一ソースで、音色選択の注記・
  `log.txt` の `patch-load: event=skipped` と**同じ 1 行**。理由の切り分けは
  [0005](0005-mixed-catalog-on-by-default.md)

## `log/log.txt` — 起動時の一覧読み込み

音色一覧の読み込みは成功も失敗も `patch-load: event=ready|skipped|error ...` の 1 行を
`%LOCALAPPDATA%\clap-mml-render-tui\log\log.txt` へ残す。

- **ログレベルの概念はこのプロジェクトに無い**ので導入していない。既存の `vvp-voicing:` /
  `play-server:` と同じ `サブシステム: key=value` 形式に揃えただけ
- `reason` は grep 用の短い綴り（`no-patches-dirs` / `patch-dirs-missing`）、
  `note` はそのまま読める 1 行

## `cmrt render-mml` — 画面を開かずにオフラインで鳴らす

`patch-roles` が「**一覧に出るか**」を数えるのに対し、こちらは「**出た音色が実際に音になるか**」を見る。
使い方は README.ja.md の「render-mml コマンド」。1 レンダリングにつき 1 行出す。

| 見るもの | 落ちる壊れ方 |
|---|---|
| `plugin=` | 引き分けの間違い（`.vvp` が Surge の添字へ落ちる、など） |
| `rms` / `silent` | 選べるのに鳴らない |
| `digest`（サンプル列の FNV-1a 64）とまとめ行の「**異なる出音 N / M**」 | **「操作は成功したが前の音のまま」** |

- **`--patch` は複数指定できる。** 1 プロセスで順に鳴らして digest を比べるためにある
- `--out-dir`（無ければ環境変数 `CMRT_TEST_WAV_OUT_DIR`）を渡したときだけ WAV を書く。
  渡さなければ 1 バイトも書かない
- サンプル列は溜めない（1 本 4 秒ステレオで 1.5MB。数百音色を一度に流すと数百 MB になる）
- `--config` の `offline_render_backend` で in-process / render server の両方を試せる。
  **render server 側も `--config` を受ける**（play-server の `clap-mml-render-server --config <PATH>`）ので、
  実ユーザーの config.toml に触らずに別プロセス経路まで通せる
- **罠: `offline_render_server_command` を引用符で始めないこと。**
  `cmd /C` が最初と最後の引用符を落とすので起動に失敗する（30 秒待って「listening しない」で落ちる）

### `--poly-check` — 和音で鳴っているかを音量で判定する

和音の RMS ÷ 単音 3 本の RMS 平均（`energy_gain`）を見る。
3 音が非干渉に重なれば `sqrt(3)` ≒ 1.73、1 音しか鳴らなければ 1.00。

**閾値は poly ≧ 1.25 / mono ≦ 1.10。間は `unclear` としてどちらとも言わない**
（黙って poly へ倒すと mono が和音行へ出る）。実測の分布は Mono 0.80〜1.10 / Poly 1.29〜2.41 で、
poly と読み違えた mono は 0 件。

- **波形の一致で見る案は却下。** mono でも単音と波形が一致せず（ノート優先・
  エンベロープ再トリガ）、グラニュラ系の poly は同じ MML でも毎回違う波形を出す
- 和音と単音は**生 MML で書く**（`t120v11'c1eg'` と `t120v11'c1'` / `'e1'` / `'g1'`）。
  chord2mml を通すと音長・音量・オクターブが単音側とずれ、判定が壊れる。
  **構成音と長さが一致することを単体テストで固定**してある

## 実プラグイン統合テスト

`#[ignore]` + 環境変数。

| 環境変数 | 用途 |
|---|---|
| `CMRT_TEST_SURGE_CLAP` | Surge XT の `.clap` パス |
| `CMRT_TEST_DEXED_CLAP` | Dexed の `.clap` パス |
| `CMRT_TEST_DEXED_CARTRIDGES` | cartridge ディレクトリ。**cartridge が 2 個以上あること**を要求する（1 個だと panic。黙って通さないための仕様） |
| `CMRT_TEST_VAPORIZER2_CLAP` | Vaporizer2 の `.clap` パス |
| `CMRT_TEST_WAV_OUT_DIR` | 耳で確かめるぶんの WAV 書き出し先。**未設定なら 1 バイトも書かない** |
| `CMRT_TEST_PLAY_SERVER_EXE` | 実 play server の実行ファイル。共有メモリ IPC を**実サーバーへ繋いで**確かめる `#[ignore]` テストが使う（`realtime-play/src/live_ipc/tests.rs`） |

Vaporizer2 の `.vvp` の置き場だけは環境変数ではなく、本番と同じ経路で config.toml の
`[plugins.Vaporizer2] patches_dirs` から読む（無ければテストが落ちる。環境依存のパスなのでコードに書かない）。

**SHM プロトコルは 2 repo に二重定義されている**（TUI の
`realtime-play/src/fast_midi_ipc/windows/protocol.rs` と play-server の
`realtime-ipc/src/windows/protocol.rs`）。片方だけ直しても**両 repo の単体テストは緑のまま通る**ので、
`CMRT_TEST_PLAY_SERVER_EXE` のテストだけがその食い違いを捕まえられる。

```
cargo test -p cmrt-realtime-play -- --include-ignored
```

テストはサーバーを自分で起こす。ポートは play server 側の
`CMRT_REALTIME_PLAY_SERVER_PORT`（config.toml より強い）で既定ポートから離すので、
**TUI を起動したままでよい**。子プロセスはテストの `Drop` が必ず落とす。

play-server で `cargo test -p cmrt-core -- --include-ignored --test-threads=1`。
**`--test-threads=1` は必須**（Vaporizer2 のテストが 2 本同時に走るとプロセスごと落ちる。
play-server `docs/adr/0013-serial-instantiation.md`）。

実プリセットのファイルしか見ない Vaporizer2 のテストは play-server の
`core-lib/src/patch_list/tests/installed.rs` にある。置き場は config.toml から読むので
`#[ignore]` ではなく通常の `cargo test -p cmrt-core` で走り（config に無ければ赤）、
実プラグインを起動しないので並列でよい。同じファイルの Dexed 側 `installed_cartridges_all_parse` は
`#[ignore]` のままで、`CMRT_TEST_DEXED_CARTRIDGES` を渡して `--include-ignored` で起こす。
**コード表と実データの食い違いは、実データを通さないと分からない。**
表に無いコードは生の 2 文字で表示され、候補から静かに外れるだけで気づけない。

## テストが実ユーザーのディレクトリを触らないようにする

- `CMRT_BASE_DIR` + `BaseDirGuard`（`core-lib/src/cache_dirs/tests.rs`）。
  **環境変数はプロセス全体なので Mutex で直列化する**
- ログは sink 注入（`cmrt_daw::set_log_sink` / `cmrt_mml_overlay::set_log_sink` を
  `app/src/main.rs` が注入）。**未注入ならログは捨てる**ので、テストでは何も書かない
- `cmrt-runtime/src/paths.rs` の `config_app_dir()` は `#[cfg(test)]` の `CMRT_BASE_DIR` フックを
  残したまま本体だけ共有 crate へ委譲している。
  **丸ごと再エクスポートにすると自身のテストが実 config を触る**

**`#[cfg(test)]` はログ・キャッシュの汚染対策にならない。** crate 自身のテストは止まるが、
app の統合テストなど他 crate のテストからその crate を通したときは `cfg(test)` が立たないので
素通りする。実ユーザーのパスへ書くものは **sink 注入**か**`CMRT_BASE_DIR` の差し替え**で塞ぐこと。

## 番人テスト（落ちたら何が壊れるか）

| テスト | 場所 | 落ちたら |
|---|---|---|
| `abstract_metadata_keeps_prefixless_categories` | `patches/src/layout/tests.rs` | 保存済みの patch 名がカテゴリを失う |
| `the_injected_log_sink_receives_the_line` | `daw/src/tests.rs` / `mml-overlay/src/tests.rs` | DAW / MML overlay のログがファイルに残らなくなる（表示は変わらないので気づきにくい） |
| `legacy_migration_moves_namespaced_daw_cache_and_keeps_intermediate_files` | `core-lib/src/cache_dirs/tests.rs` | 旧キャッシュ掃除が render-server の中間ファイルを消す |
| `every_row_role_appears_in_the_role_table` | `app/src/tui/patch_role_report/tests.rs` | `patch-roles` の用途一覧の追加漏れを検出できなくなる |
| `the_breakdown_counts_every_candidate_once_per_plugin` | 同上 | 候補数のプラグイン別内訳が過不足を出す（0 件のプラグインが消える） |
| `a_vvp_patch_goes_to_vaporizer2_not_to_the_other_state_file_plugin` | `tui-core/src/patch_plugins/tests.rs` | `.vvp` が Surge の添字へ落ちる |
| `each_patch_form_reports_its_own_plugin` | `app/src/render_mml/tests.rs` | オフライン経路の引き分けが壊れた |
| `the_poly_check_notes_are_exactly_the_notes_of_the_chord` | 同上 | `--poly-check` の判定が黙って壊れる（和音と単音が 1 対 1 でなくなる） |
| `a_patch_name_round_trips_through_the_mml_head_json` | 同上 | **アポストロフィ入りの音色名**で MML が壊れる |
| `the_mml_overlay_sees_the_vvp_patches` | `app/src/tui/tests/vaporizer2_screens.rs` | 画面が共有一覧ではなく自前で音色を走査しはじめた |
| `namespace_differs_for_vaporizer2_too` | `core-lib/src/cache_dirs/tests.rs` | cache 名前空間が 3 つめで分かれない |
| `the_default_config_lists_every_vaporizer2_category_code` | `cmrt-runtime/src/tests.rs` | 生成する config.toml のカテゴリコード表が古びた |

**ADR に書いた番人テスト名が実装の改名で古びていないか**は
`python scripts/check_adr_test_names.py` が機械で見る。

play-server 側の番人テストは play-server `docs/adr/0012-measured-baselines.md`。

## 実測から残す判断

- `cmrt patch-roles` の候補数は、プラグインを足すと**足したプラグインの件数ぶんだけ増える**のが正。
  Dexed は絞らないので全件が全用途へ乗る。合わないときの既知の原因: **カテゴリ名はプラグインを
  跨いで素の文字列比較で当たる**ので、`.vvp` が Surge の添字へ落ちると Surge の同名カテゴリに紛れる
- `cmrt render-mml` で無音になる Vaporizer2 プリセットは名前に `MPE` を含む物で、note dialect が
  MIDI のみで per-note pitch / pressure を送る口が無いため（play-server 側 ADR 0001）。こちらのバグではない
- **Dexed だけ 2 バックエンドで RMS が完全一致する。** Surge と Vaporizer2 は位相がランダムなので
  数字がずれる。「バックエンドを替えても同じ音か」を digest で見たいときは Dexed を使う
- `cmrt build-voicing-cache` が probe できるのは Surge の patch だけで、cartridge は probe 対象外
- **Surge は同一プロセスで同じ MML を 2 回レンダリングしてもサンプルが一致しない**
  （初期パッチのランダム位相などプラグイン側の性質）。
  **「出力が 1 bit も変わらないこと」を回帰テストの条件にしてはいけない。**
  代わりに「Surge が CLAP note 経路のままであること」を capability で固定する
- 音の同一性の閾値 `SAME_SOUND_TOLERANCE = 0.001`（play-server `core-lib/src/render/tests/cartridge.rs`）。
  **同じ program を選び直しても 2e-5 程度の差が残る**（LFO 位相などプラグイン内部の状態）
