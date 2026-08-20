# ADR 0004: 音色を無指定にした行は既定プラグインで鳴る / cache 名前空間

- 状態: 採用（2026-08-20）
- 関連: [0003](0003-mml-patch-key.md) / [0005](0005-mixed-catalog-on-by-default.md)

## 決定

1. **音色を「無指定」へ戻した行が鳴るプラグインは、常に既定プラグイン（`active_plugin`）1 つ。**
2. cache の名前空間は **`plugin_path` のファイル名（拡張子なし）** 1 本で決める。
3. cache 名前空間の `OnceLock`（`core-lib/src/cache_dirs.rs`）は**混在でも無改修のまま正しい**。

## 理由 (1): なぜ既定プラグイン固定でよいか

無指定の行に「どのプラグインか」を書く場所は定義上どこにもない。既定へ倒すのが唯一一貫する。

## 理由 (2): なぜ `plugin_path` のファイル名か

cache key は `cmrt_history::daw_cache_mml_hash(mml)` ＝ **MML 文字列そのものの hash**。
キーへプラグインを混ぜるには 2 repo 合わせて 40 箇所ある hash 呼び出し全部へ引数を足すことになり、
`DawCachedMeasure::normalize()` のように config を持てない場所も含む。
そこで**ディレクトリを分ける**ことで誤ヒットを断った。

`active_plugin` や `plugin_id` を名前空間にすると、`active_plugin = 'Surge XT'` と書いた config と、
トップレベル `plugin_path` だけの旧 config が**別ディレクトリになってしまう**（旧 config には
`plugin_id` が無いため）。**`plugin_path` はどちらの書き方でも必ず埋まる。**

```
config_local/clap-mml-render-tui/
  daw/
    daw_cache.mid          ← render-server の中間ファイル。従来どおり直下
    daw_cache.wav
    Surge XT/track0_meas1.wav
    Dexed/track0_meas1.wav
  notepad_cache/
    Surge XT/0123abcd....wav
    Dexed/0123abcd....wav
```

## 理由 (3): なぜ混在でも `OnceLock` 1 つでよいか

衝突の発生条件を正確に見ると、混在でも成立する:

- **音色を指定した行は衝突しない。** MML 先頭に `{"Surge XT patch": "..."}` が埋まっており、
  Surge の `.fxp` パスと Dexed の `.syx/NN` パスで**文字列そのものが違う**
- **衝突しうるのは「音色を指定していない行」だけ**（Surge の Init Saw と Dexed の INIT VOICE）
- 決定 (1) により、無指定行が鳴るプラグインは 1 つに固定される

**根拠は「1 プロセス 1 プラグインだから」ではない**（それは古い理由で、混在後は嘘になる）。
**「音色無指定の行が鳴るプラグインが 1 つだから」**が正しい根拠。

## 旧キャッシュの掃除を足した理由

置き場を変えると、旧ファイルは**誰も読まないのに LRU の上限計算からも外れ、二度と消えなくなる**。
起動時に 1 回だけ掃除する。消すのはキャッシュ専用のファイル名の形
（`daw/track{数字}_meas{数字}.wav` と `notepad_cache/{16 桁 hex}.wav`）だけで、
`daw_cache.mid` / `daw_cache.wav` には触れない。

## 罠

- **`OnceLock` は main が `config::load()` の直後に `init_cache_plugin_namespace()` を呼んで決める。**
  ここより前にキャッシュ API を呼ぶコードを足すと、黙って `unknown-plugin` へ書く
- **`remove_legacy_unnamespaced_caches()` は実際にファイルを消す。**
  テストから呼ぶときは必ず `CMRT_BASE_DIR` を一時ディレクトリへ寄せる
- **`ensure_notepad_cache_dir()` は名前が同じまま置き場が変わった。**
  呼び出し側は無改修だが、古いパスを前提にしたテストを書くと通らない
- session state 側は無改修でよい。`DawApp::restore_cache_from_history()` は
  「MML hash が一致」だけでなく**WAV が実在して spec も一致する**ことを要求するので、
  置き場が変わればファイルが見つからず `Pending` へ落ちる

## 壊れたら気づく場所

- `core-lib/src/cache_dirs/tests.rs::legacy_cleanup_keeps_render_server_intermediate_files`
  — 旧キャッシュ掃除が render-server の中間ファイルを消していないこと
