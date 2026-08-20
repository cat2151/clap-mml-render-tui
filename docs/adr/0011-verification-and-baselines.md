# ADR 0011: 検証手段と実測ベースライン

- 状態: 採用（2026-08-20）
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

**注意**: `patch-roles` は config.toml をそのまま読む。プラグインを替えて試すには
`active_plugin` を書き換える必要がある。**書き換えたら必ず戻すこと。**
drum 行の表は track 7 固定（track 4 は drum 行が 1 つで役割が抽選なので表に出せない）。

## 実プラグイン統合テスト

`#[ignore]` + 環境変数。

| 環境変数 | 用途 |
|---|---|
| `CMRT_TEST_SURGE_CLAP` | Surge XT の `.clap` パス |
| `CMRT_TEST_DEXED_CLAP` | Dexed の `.clap` パス |
| `CMRT_TEST_DEXED_CARTRIDGES` | cartridge ディレクトリ。**cartridge が 2 個以上あること**を要求する（1 個だと panic。黙って通さないための仕様） |

play-server で `cargo test -p cmrt-core -- --include-ignored --test-threads=1`。

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

### そのほか

- `cmrt build-voicing-cache`（混在）: `patch 4064 件中 probe できるのは 3008 件`
  — cartridge の 1,056 件は probe 対象外
- **Surge は同一プロセスで同じ MML を 2 回レンダリングしてもサンプルが一致しない**
  （初期パッチのランダム位相などプラグイン側の性質。host の変更とは無関係）。
  **「出力が 1 bit も変わらないこと」を回帰テストの条件にしてはいけない。**
  代わりに「Surge が CLAP note 経路のままであること」を capability で固定する
- 音の同一性の閾値 `SAME_SOUND_TOLERANCE = 0.001`。
  **同じ program を選び直しても 2e-5 程度の差が残る**（LFO 位相などプラグイン内部の状態）
