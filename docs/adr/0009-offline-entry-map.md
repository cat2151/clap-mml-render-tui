# ADR 0009: オフラインレンダリングは MML ごとに entry を引き分ける

- 状態: 採用（2026-08-20）
- 関連: [0001](0001-patch-string-decides-the-plugin.md) / [0005](0005-mixed-catalog-on-by-default.md)

## 決定

「MML 1 本 → その音色のプラグイン」の引き当てを、**レンダリング経路すべてへ通す。**
`entry_ptr: usize` を裸で持ち回すのをやめ、型を付けた。

| 型 | 場所 | 役割 |
|---|---|---|
| `PluginEntries` | `offline-render/src/plugin_entries.rs` | ロード済み entry を**カタログの並び**で持つ。`0` は「in-process では鳴らせない」。`is_available()` が旧 `entry_ptr != 0` |
| `InProcessPlugins` | `offline-render/src/in_process.rs` | **MML → (entry, `CoreConfig`)**。`for_mml` が公開 API |
| `core_config_for_plugin` | `core-lib/src/core_config.rs` | カタログ 1 プラグインぶんの `CoreConfig` |
| `embedded_patch_ref` | `cmrt_core` の再輸出 | MML 先頭 JSON の音色を**解決せず**返す |

**音色を無指定にした MML は必ず既定プラグイン（カタログの先頭）。**

## 3 プラグイン目は無改修で通った（2026-08-22）

`PluginEntries` も `InProcessPlugins` も**カタログの並び**で持つだけなので、
Vaporizer2 を足しても引き分けのコードは 1 バイトも変わらなかった
（`.vvp` の判別が `PatchPlugins::index_for_patch()` へ入っているため）。
play-server の render server 側（`kind_for_patch()` / `extract_patch_from_json()`）も同じ。

実測（`cmrt render-mml`、in-process / render server の両バックエンド）:

| patch | 引き分けたプラグイン | rms | 無音 |
|---|---|---|---|
| `patches_factory/Pads/Pad 1.fxp` | Surge XT | 0.0335 | no |
| `Dexed_01.syx/00 Say Again.` | Dexed | 0.0316 | no |
| `AT Ambience 1.vvp` | **Vaporizer2** | 0.0477 | no |

**`.vvp` に固有の危険はアポストロフィ**（`AT I'll House Your Grains.vvp` など 2 件）。
MML の和音記法が `'...'` なので、先頭 JSON を素の文字列連結で作ると MML 側が壊れる。
**JSON を通していれば壊れない**ことを往復テストで固定してある
（`app/src/render_mml/tests.rs::a_patch_name_round_trips_through_the_mml_head_json`）。

## なぜ「静かに間違う」ので全経路に通す必要があるか

**Surge のインスタンスへ DX7 の SysEx を送ると、Surge は理解できない 163 byte を
黙って無視する。エラーにならない。**

つまり引き分けを通し忘れた経路は「**Dexed の音色を選んだのに前の音のまま、操作は成功扱い**」
という、いちばん気づきにくい壊れ方をする。

（play-server 側では `ensure_cartridge_capable` で照合を足してあるので、いまはエラーになる。
ただし**逆方向（Dexed へ `.fxp` の state load）には照合を入れていない**。
そちらはプラグインが state load を失敗させるので、黙って無視されることがないため。）

## 判別は解決より先

`embedded_patch_ref` は音色を**解決せずに**返す。解決の基点がプラグインごとに違う
（[0006](0006-per-profile-relative-base.md)）ので、**判別を解決より先に行う必要がある。**

同じ理由で `PreparedOfflineRender::InProcess { prepared, plugin }` は
**prepare 時に決めた添字を持ち回す。** prepare 済み MML は先頭 JSON が絶対パスへ
書き換わっているので、レンダー時に引き直すと判別材料が変わってしまう。

## 見落としやすい 2 経路

`app/src/main.rs` がロードする entry は offline render だけでなく
**`--server`（`app/src/server.rs`）と CLI モードの `mml_to_play`** も使っていた。
どちらも MML の音色でプラグインが決まる経路なので、同じ引き当てを通してある。

production の `load_entry` 呼び出しはこの経路と play-server 側の 3 箇所だけ。残りは全部テスト。

## 既知の表示上の問題（実害なし）

`cmrt --server` の音色無指定ログが `patch=(Init Saw)` と Surge の名前で出る。
既定プラグインが Dexed でもこの文字列になる（混在対応より前からの表示）。

## 壊れたら気づく場所

- `offline-render/src/in_process/tests.rs` — cartridge の MML が Dexed の entry へ、
  `.fxp` の MML が Surge の entry へ、無指定が既定プラグインへ行くこと
- `app/src/render_mml/tests.rs::each_patch_form_reports_its_own_plugin` — 3 形すべての引き分け
  （**`.vvp` が Surge の添字へ落ちないこと**）
- `app/src/render_mml/tests.rs::an_unspecified_patch_reports_the_default_plugin` — 無指定は必ず先頭
