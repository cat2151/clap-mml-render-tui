# ADR 0011: 検証手段と実測ベースライン

- 状態: 採用（2026-08-20 / 2026-08-22 に `cmrt render-mml` を追加）
- 関連: すべての ADR

## `cmrt patch-roles` — 画面を開かずに wheel の候補を数える

grid sequencer の PATCH 欄の wheel が引く候補は「config の用途別カテゴリ」「patch 一覧」
「patch ごとの mono/poly 判定」の 3 つで決まる。**プラグインを替えたときにどれか 1 つでも
噛み合わないと wheel が無反応になる**が、それを見るには画面を開いて全行の wheel を
回すしかなかった。これを画面なしで数える。

- 8 つの用途（Chord / Bass / Arpeggio / Free / Kick / Snare / HiHat / Percussion）ごとの候補数と例
- chord mode ON / OFF それぞれの、行ごとの用途と候補数
- **候補 0 件の行があれば、その行を挙げて終了コード 1** で終わる（スクリプトから判定できる）

**設計の肝**: `grid-sequencer/src/patch_role.rs` の `row_patch_role()` に行→用途の対応を
1 か所へ寄せ、**候補を数える述語を wheel と CLI で 1 本にした**。
ここを patch_selector 側へ戻すと「CLI が通っても画面が通らない」状態を作れてしまう。

**別の config で試すには `--config` を渡す**（既定は実ユーザーの config.toml）:

```
cmrt patch-roles --config /path/to/try.toml
```

`Config::load_from_path()` を通るので、**既定の置き場を作りに行かない＝実ユーザーの
config.toml には 1 バイトも触らない**。`active_plugin` や `[plugins.*]` を試すたびに
実ファイルを書き換えて戻す運用は、戻し忘れがそのまま本番の設定事故になる。

なお **`%LOCALAPPDATA%` を環境変数で差し替えても効かない**。`dirs::config_local_dir()` は
Windows では Known Folder API を引くので、env var は無視される（実測）。

drum 行の表は track 7 固定（track 4 は drum 行が 1 つで役割が抽選なので表に出せない）。

**カタログが 2 プラグイン以上のときは、用途ごとの候補数にプラグイン別の内訳も出る。**
合計だけでは「あるプラグインの音色がその役へ 1 件も出ていない」が他の数千件に埋もれて見えない。

```text
[用途ごとの候補数]
  Chord         1997 件  例: AT Ambience 1.vvp | AT Ambience 2.vvp ...
                       内訳: Dexed 1056 / Surge XT 751 / Vaporizer2 190
```

- 引き分けは `PatchPlugins::index_for_patch()` ＝ **PATCH 欄の wheel と同じ述語**を通す
- **候補 0 件のプラグインも省略せず 0 と出す**（省略すると「0 件」と「数えていない」の
  区別が付かない）
- 「voicing 判定」欄にプラグインごとの `VoicingPolicy` が出る（`Sources` / `VvpHeader` /
  `AssumePoly`。[0008](0008-voicing-per-patch.md)）

## `cmrt render-mml` — 画面を開かずにオフラインで鳴らす

```
cmrt render-mml [--config PATH] [--patch DISPLAY]... [--out-dir DIR] [--poly-check] [MML]
```

`patch-roles` が「**一覧に出るか**」を数えるのに対し、こちらは「**出た音色が実際に音になるか**」を見る。
1 レンダリングにつき 1 行:

```text
render patch='AR Accent Arp.vvp' plugin=Vaporizer2 frames=192000 duration_ms=4000
       peak=0.3255 rms=0.034750 silent=no digest=7e896ce776d2ede8 elapsed_ms=558
       patch_name='AR Accent Arp.vvp' wav=-
```

| 見るもの | 落ちる壊れ方 |
|---|---|
| `plugin=` | 引き分けの間違い（`.vvp` が Surge の添字へ落ちる、など） |
| `rms` / `silent` | 選べるのに鳴らない |
| `digest`（サンプル列の FNV-1a 64）とまとめ行の「**異なる出音 N / M**」 | **「操作は成功したが前の音のまま」** |

- **`--patch` は複数指定できる。** 1 プロセスで順に鳴らして digest を比べるためにある
- `--out-dir`（無ければ環境変数 `CMRT_TEST_WAV_OUT_DIR`）を渡したときだけ WAV を書く。
  **耳で確かめたいぶんはここへ溜めて後で聴く**。渡さなければ 1 バイトも書かない
- サンプル列は溜めない（1 本 4 秒ステレオで 1.5MB。460 音色を一度に流すと 700MB になる）
- `--config` の `offline_render_backend` で in-process / render server の両方を試せる。
  **render server 側も `--config` を受ける**（play-server の `clap-mml-render-server --config <PATH>`）ので、
  実ユーザーの config.toml に触らずに別プロセス経路まで通せる
- **罠: `offline_render_server_command` を引用符で始めないこと。**
  `cmd /C` が最初と最後の引用符を落とすので起動に失敗する（30 秒待って「listening しない」で落ちる）

### `--poly-check` — 和音で鳴っているかを音量で判定する

和音の RMS ÷ 単音 3 本の RMS 平均（`energy_gain`）を見る。
3 音が非干渉に重なれば `sqrt(3)` ≒ 1.73、1 音しか鳴らなければ 1.00。

**閾値は poly ≧ 1.25 / mono ≦ 1.10。間は `unclear` としてどちらとも言わない**
（黙って poly へ倒すと mono が和音行へ出る）。

- 実測は Mono 14 件が 0.80〜1.10 / Poly 10 件が 1.29〜2.41。**poly と読み違えた mono は 0 件**
- **波形の一致で見る案は実測で外れた。** mono でも単音と波形が一致せず（ノート優先・
  エンベロープ再トリガ）、グラニュラ系の poly は同じ MML でも毎回違う波形を出す
- 和音と単音は**生 MML で書く**（`t120v11'c1eg'` と `t120v11'c1'` / `'e1'` / `'g1'`）。
  chord2mml を通すと音長・音量・オクターブが単音側とずれ、判定が壊れる。
  ずれようがない形にしたうえで、**構成音と長さが一致することを単体テストで固定**してある

## 実プラグイン統合テスト

`#[ignore]` + 環境変数。

| 環境変数 | 用途 |
|---|---|
| `CMRT_TEST_SURGE_CLAP` | Surge XT の `.clap` パス |
| `CMRT_TEST_DEXED_CLAP` | Dexed の `.clap` パス |
| `CMRT_TEST_DEXED_CARTRIDGES` | cartridge ディレクトリ。**cartridge が 2 個以上あること**を要求する（1 個だと panic。黙って通さないための仕様） |
| `CMRT_TEST_VAPORIZER2_CLAP` | Vaporizer2 の `.clap` パス |
| `CMRT_TEST_VAPORIZER2_PRESETS` | `.vvp` の置き場（個人のパスなのでコードに書かない） |
| `CMRT_TEST_WAV_OUT_DIR` | 耳で確かめるぶんの WAV 書き出し先。**未設定なら 1 バイトも書かない** |

play-server で `cargo test -p cmrt-core -- --include-ignored --test-threads=1`。
**`--test-threads=1` は必須**（Vaporizer2 のテストが 2 本同時に走るとプロセスごと落ちる。
play-server `docs/adr/0013-serial-instantiation.md`）。

**TUI 側にも `CMRT_TEST_VAPORIZER2_PRESETS` を使う `#[ignore]` テストが 2 本ある。**
どちらも実プラグインは要らず**プリセットのファイルしか見ない**ので並列でよい:

```
cargo test -p cmrt-patches -- --include-ignored --nocapture
    → ファイル名 → カテゴリの表が実データと合っているか（460 件）
cargo test -p clap-mml-render-tui tui::voicing -- --include-ignored --nocapture
    → 460 件すべてで m_uPolyMode が読めるか・先読みの実時間
```

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
**app の統合テストなど他 crate のテストからその crate を通したときは `cfg(test)` が立たない**ので
素通りする（実際に踏んだ）。実ユーザーのパスへ書くものは **sink 注入**か
**`CMRT_BASE_DIR` の差し替え**で塞ぐこと。

## 番人テスト（落ちたら何が壊れるか）

| テスト | 場所 | 落ちたら |
|---|---|---|
| `free_keeps_every_patch_when_the_chord_categories_are_empty` | `patches/src/selection/tests.rs` | Dexed の Free 行（chord mode off の全行）が候補 0 件になる |
| `the_builtin_dexed_profile_does_not_narrow_the_patch_roles` | `cmrt-runtime/src/plugin_profile/tests.rs` | Dexed に Surge のカテゴリ名が復活する |
| `the_surge_profile_keeps_the_top_level_patch_categories` | 同上 | 既存 Surge ユーザーの挙動が変わる |
| `a_profile_can_narrow_the_patch_roles_by_itself` | 同上 | `[plugins.*]` のカテゴリ指定が効かなくなる |
| `a_profile_for_the_default_plugin_still_contributes_its_patch_roles` | `cmrt-runtime/src/core_config/tests.rs` | 既定 config 末尾の案内ブロックが嘘になる |
| `a_prefixless_surge_name_reads_the_same_either_way` | `patches/src/layout/tests.rs` | 保存済みの patch 名がカテゴリを失う |
| `the_injected_log_sink_receives_the_line` | `daw/src/tests.rs` / `mml-overlay/src/tests.rs` | DAW / MML overlay のログがファイルに残らなくなる（表示は変わらないので気づきにくい） |
| `legacy_cleanup_keeps_render_server_intermediate_files` | `core-lib/src/cache_dirs/tests.rs` | 旧キャッシュ掃除が render-server の中間ファイルを消す |
| `every_row_role_appears_in_the_role_table` | `app/src/tui/patch_role_report/tests.rs` | `patch-roles` の `ALL_ROLES` 追加漏れを検出できなくなる |
| `the_breakdown_counts_every_candidate_once_per_plugin` | 同上 | 候補数のプラグイン別内訳が過不足を出す（0 件のプラグインが消える） |
| `a_vvp_patch_goes_to_vaporizer2_not_to_the_other_state_file_plugin` | `tui-core/src/patch_plugins/tests.rs` | `.vvp` が Surge の添字へ落ちる |
| `the_vaporizer2_categories_are_not_the_surge_ones` | `cmrt-runtime/src/patch_roles/tests.rs` | 「Surge のぶんをコピーした」間違い |
| `the_lowercased_form_lands_in_the_same_category` | `patches/src/vaporizer2/tests.rs` | 同じカテゴリが見出し 2 つに割れる（グループのキーは小文字側から作る） |
| `every_installed_preset_lands_in_a_known_category` | 同上（`#[ignore]`） | コード表が実データから外れた（460 件） |
| `an_unreadable_preset_stays_undecided` | `app/src/tui/voicing/vvp_voicings/tests.rs` | 読めない `.vvp` が poly へ倒れる（Mono が和音行へ出る） |
| `each_patch_form_reports_its_own_plugin` | `app/src/render_mml/tests.rs` | オフライン経路の引き分けが壊れた |
| `the_poly_check_notes_are_exactly_the_notes_of_the_chord` | 同上 | `--poly-check` の判定が黙って壊れる（和音と単音が 1 対 1 でなくなる） |
| `a_patch_name_round_trips_through_the_mml_head_json` | 同上 | **アポストロフィ入りの音色名**で MML が壊れる |
| `the_mml_overlay_sees_the_vvp_patches` | `app/src/tui/tests/vaporizer2_screens.rs` | 画面が共有一覧ではなく自前で音色を走査しはじめた |
| `namespace_differs_for_vaporizer2_too` | `core-lib/src/cache_dirs/tests.rs` | cache 名前空間が 3 つめで分かれない |
| `the_default_config_lists_every_vaporizer2_category_code` | `cmrt-runtime/src/tests.rs` | 生成する config.toml のカテゴリコード表が古びた |

**ADR に書いた番人テスト名が実装の改名で古びていないか**は
`python scripts/check_adr_test_names.py` が機械で見る。

play-server 側の番人テストは play-server `docs/adr/0012-measured-baselines.md`。

## 実測ベースライン（退行検知用 / 2026-08-20）

### `cmrt patch-roles` の候補数

| 構成 | patch 件数 | 候補数 |
|---|---|---|
| Surge XT 単独 | 3,008 | Chord 751 / Bass 408 / Arpeggio 1003 / Free 2257 / Kick 50 / Snare 45 / HiHat 22 / Percussion 122 |
| Dexed 単独（カテゴリ 7 項目とも空） | 1,056 | 8 用途とも 1,056。chord mode ON/OFF とも行0〜行6 すべて 1,056 |
| **混在（既定 Dexed + Surge XT）** | **4,064** | Chord 1807 / Bass 1464 / Arpeggio 2059 / Free 3313 / Kick 1106 / Snare 1101 / HiHat 1078 / Percussion 1178 |

- 混在の値はすべて **Surge 単独の値 + 1,056**（Dexed は絞らないので全件）に一致する
- Surge 単独の `Free 2257 = 3008 − 751` という検算が立つ
- 実物 cartridge は 33 files × 32 program = **1,056 program**

**Vaporizer2 を足した config**（`cmrt patch-roles --config <PATH>` / 2026-08-22）:

| | 既定 config | `[plugins.Vaporizer2]` 入り |
|---|---|---|
| カタログのプラグイン | Dexed / Surge XT | + **Vaporizer2** |
| patch 件数 | 4064 | **4524**（+ `.vvp` 460） |
| Chord / Bass / Arpeggio / Free | 1807 / 1464 / 2059 / 3313 | **1983 / 1567 / 2123 / 3597** |
| Kick / Snare / HiHat / Percussion | 1106 / 1101 / 1078 / 1178 | 1112 / 1102 / 1079 / 1179 |
| Vaporizer2 の内訳 | — | chord **176** / bass 103 / arpeggio 64 / drum 9（6+1+1+1） |
| voicing 判定（Vaporizer2） | — | **`VvpHeader`** |

読み方 4 つ:

- **既定 config は 1 件も動いていない。** Vaporizer2 は音色置き場の既定値を持たないので
  カタログに載らない（[0005](0005-mixed-catalog-on-by-default.md)）
- chord の内訳 **176 = カテゴリ 190 − そのカテゴリ内の Mono 14**。
  `.vvp` のヘッダを読む方針（[0008](0008-voicing-per-patch.md)）が効いている数字で、
  **preset ディレクトリを直接数え直した別経路と完全に一致した**
- **Free は 460 − 190 = 270。** 用途別カテゴリの層 3 が入ったぶんだけ Free から抜ける
  （[0007](0007-patch-role-defaults-three-layers.md)）
- Arpeggio が +64 ではなく +62 に見えた時期があったのは、`.vvp` が Surge の添字へ
  落ちていて Surge の `Brass` カテゴリに 2 件たまたま当たっていたため。
  **カテゴリ名はプラグインを跨いで素の文字列比較で当たる**ので、添字を分けるまでこうなる

### `cmrt render-mml` の実測（2026-08-22 / in-process / 実プリセット 460 件）

```
レンダリング数 460 / 無音 5 件 / 異なる出音 455 / 460
合計 328 秒（1 件平均 714ms・最大 9727ms = PD Emily.vvp の 17MB）
```

- **無音だった 5 件はすべて名前に `MPE` を含む。** Vaporizer2 の note dialect は MIDI のみで
  per-note pitch / pressure を送る口が無い（play-server 側 ADR 0001）。**こちらのバグではない**
- 455 は「無音 5 件が同じ digest を共有した」ぶん（454 通り + 無音 1 通り）。
  **鳴った 455 件は 1 件も重複していない**
- **Dexed だけ 2 バックエンドで RMS が完全一致する**（0.031626）。Surge と Vaporizer2 は
  位相がランダムなので数字がずれる。「バックエンドを替えても同じ音か」を digest で見たいときは Dexed を使う

### そのほか

- `cmrt build-voicing-cache`（混在）: `patch 4064 件中 probe できるのは 3008 件`
  — cartridge の 1,056 件は probe 対象外
- **Surge は同一プロセスで同じ MML を 2 回レンダリングしてもサンプルが一致しない**
  （初期パッチのランダム位相などプラグイン側の性質。host の変更とは無関係）。
  **「出力が 1 bit も変わらないこと」を回帰テストの条件にしてはいけない。**
  代わりに「Surge が CLAP note 経路のままであること」を capability で固定する
- 音の同一性の閾値 `SAME_SOUND_TOLERANCE = 0.001`。
  **同じ program を選び直しても 2e-5 程度の差が残る**（LFO 位相などプラグイン内部の状態）
