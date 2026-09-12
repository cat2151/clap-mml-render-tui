# ADR 0002: config はプラグインプロファイル方式

- 状態: config 選択方式は [0014](0014-fixed-surge-primary-plugin.md) により廃止
- 関連: [0005](0005-mixed-catalog-on-by-default.md) / [0007](0007-patch-role-defaults-three-layers.md) / [0014](0014-fixed-surge-primary-plugin.md)

> 以下は当時の決定記録。`active_plugin` とトップレベルの plugin 設定は現在の入力構文ではない。

## 決定

`active_plugin` 1 行 + `[plugins.<名前>]` テーブルでプラグインを切り替える。
`preset_dirs` のような新キーは**作らず**、プロファイル内の `patches_dirs` が
そのまま「今の plugin の音色置き場」を意味する。

## 理由

新キーを足すと「トップレベルと両方指定されたとき」の曖昧さ処理が要る。
既存キーの意味を広げるほうが宣言も分岐も増えない。

**実装のキモ**: プロファイルは `Config::load()` の中でトップレベルフィールドへ焼き込む。
これにより `cfg.plugin_path` / `cfg.patches_dirs` の読み手は両 repo とも全て無改修で済んだ。

> ただし**用途別カテゴリ 7 項目だけは焼き込みをやめた**。理由は [0007](0007-patch-role-defaults-three-layers.md)。

## 採らなかった案

- **トップレベルのキーを増やす**: `#[serde(default = ...)]` で必ず値が入るため、
  deserialize 後に「ユーザーが書いた」と「既定値が入った」を区別できない
- **TUI の `Config` へ `#[serde(flatten)] server: ServerConfig` を埋める**:
  `cfg.plugin_path` の読み手が **137 か所**、`Config { .. }` の構造体リテラルが **54 か所**
  すべて書き換えになる。得られるのは「同じ TOML キーの宣言が 1 か所になる」だけで、
  **重複するのは宣言 10 個ほど、ロジックは 0**

## 罠

- **`active_plugin` を書かずトップレベル `plugin_path` だけの config はプロファイルを通らない。**
  `resolve_active_plugin_profile` は `active_plugin` が `None` なら `Ok(None)` で戻る。
  組み込みの既定値（Dexed のカテゴリ空など）が効かない
- そのため「既定プラグインと同じものを指すプロファイル」は重複排除で丸ごと捨てられていた。
  **`patch_roles` だけは拾う**形にしてある。
  **`[plugins.*]` へ他の項目を足すときは同じ穴を踏む**ので、既定プラグインぶんの
  拾い直しが要るかを必ず確かめること
- **`ServerConfig` に項目を足すときは TUI の `Config` とキー名を必ず揃える。**
  別宣言なのでコンパイラは教えてくれず、片方だけ増やしても**未知キーとして黙って無視される**
- **プロファイルの焼き込みは無条件の代入。** `apply_active_plugin_profile()` の
  `self.patches_dirs = profile.patches_dirs;` は `None` でも代入するので、
  **`patches_dirs` を持たない組み込みプロファイル（Vaporizer2）を `active_plugin` にすると、
  トップレベルに書いてある音色置き場が消えて音色 0 件になる。** これは正しい倒れ方で、
  消えないと Surge の `.fxp` が Vaporizer2 のインスタンスへ送られる
  （[0001](0001-patch-string-decides-the-plugin.md) の弱点の実害）

## 壊れたら気づく場所

- `cmrt-runtime/src/plugin_profile/tests.rs::retired_patch_role_overrides_are_rejected`
  — `[plugins.*]` の用途別カテゴリ指定は廃止済みで、書けば設定エラーになること
