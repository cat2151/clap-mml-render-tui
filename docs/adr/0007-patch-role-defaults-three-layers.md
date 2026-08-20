# ADR 0007: 用途別カテゴリの既定値は 3 層で解決する

- 状態: 採用（2026-08-20）
- 関連: [0002](0002-config-plugin-profiles.md) / [0005](0005-mixed-catalog-on-by-default.md) /
  [0010](0010-two-repo-layout.md)

## 決定

用途別 7 項目（`chord_patch_categories` / `bass_...` / `arpeggio_...` / `drum_...` /
`kick_patch_keywords` / `snare_...` / `hihat_...`）の解決を **3 層**にする。

| 層 | 中身 | 効く範囲 |
|---|---|---|
| 1 | `[plugins.<名前>]` に書かれた項目 | そのプラグイン |
| 2 | config トップレベルに書かれた項目（**レガシー綴り**） | **既定プラグインだけ** |
| 3 | そのプラグインの組み込み既定（`PatchRoles::builtin_for`） | そのプラグイン |

層 3 は Surge XT なら `cmrt_patches::surge_xt::DEFAULT_*`、**それ以外は空（＝絞らない）**。
判定は `is_surge_xt_plugin`（`plugin_id`、無ければ `plugin_path` のファイル名）。

## 解こうとした問題

config.toml のトップレベル 7 項目の既定値が **Surge XT のカテゴリ名**で、それが
「プロファイルが書いていない項目の土台」になっていた。組み込みの Dexed が無事なのは
プロファイルが 7 項目すべてを空で明示しているから（`PatchRoleFilters::unfiltered()`）で、
**プロファイルを持たない `[plugins.my_synth]` をカタログへ足すと Surge のカテゴリで絞られて
候補が全滅する。**

## なぜ「トップレベルを空にする」ではなく「層 2 の範囲を狭める」か

**既存 config.toml の書き換えが要らない。**

- 既存ユーザー（トップレベルに Surge のカテゴリ名がある）→ 既定プラグインに対して
  今までどおり効く。**結果は 1 件も変わらない**
- そこへ `[plugins.my_synth]` を足す → 層 2 が届かないので層 3（空）へ落ちる。**問題が直る**
- `active_plugin = 'my_synth'` にした場合も直る。この経路は「トップレベルを空にする」だけでは
  直らなかった（既定プラグインには層 2 が届くため）ので、**層 3 が要る**

## なぜ Surge の既定を「組み込みプロファイル」へ書けないか

`builtin_plugin_profiles()` は play-server の `server-config` にあるが、
Surge のカテゴリ名の実体は **TUI repo の `cmrt-patches`**。
組み込みプロファイルへ値を書くと **play-server が TUI の crate を引く**ことになり、
[0010](0010-two-repo-layout.md) で消したばかりの逆向きの辺が復活する。

なお `patch_roles` は `server-config` に**宣言があるだけ**で、play-server のコードは 1 か所も
読んでいない。**これは初めから TUI 専用のデータ。**

## `None` と `[]` は別物

`PluginProfile` は `#[serde(flatten)]` で `PatchRoleFilters` を持ち、各項目は `Option<Vec<String>>`。

- **`None` = 書かれていない**（下の層へ落ちる）
- **`[]` = 「絞らない」という明示の指定**

この区別が要るので `Option` を剥がせない。組み込みプロファイルは
`Surge XT` が `default()`（全部 `None`）、`Dexed` が `unfiltered()`（全部 `[]`）。

`Config` 側も `Vec<String>` から `#[serde(flatten)] top_level_patch_roles: PatchRoleFilters` へ変えた。
`Vec<String>` のままだと「書かれていない」と「`[]`」を区別できず、層 2 を狭める意味が出ない。
（`Config` に `deny_unknown_fields` が無いので flatten を足せた。付いていたら serde の制約で使えない。）

## カテゴリを空にすると `Free` 行が全滅する論理

`matches_role` の `PatchRole::Free`（chord mode off のときの**全行**）は元々
`!(in_category && is_poly)` だった。カテゴリを空にすると `patch_matches_categories` が
全 patch で true、`AssumePoly`（[0008](0008-voicing-per-patch.md)）で `is_poly` も全 patch で true、
→ `!(true && true)` = false で**全行の候補が 0 件**になる。

```rust
PatchRole::Free => filter.categories.is_empty() || !(in_category && voicing.is_poly(display)),
```

意味づけ: **カテゴリが空＝「どれを chord 行へ回すか」が定義されていない**のだから、
Free は何も避けない。

## 生成する config.toml の形（ファイル末尾のコメント済みプロファイル）

トップレベルの 7 項目は**値として書き出さなくなった**。代わりにファイル末尾へ
**コメント済みの `[plugins."Surge XT"]` ブロック**を置き、Surge の既定値をそこに見せる。

- **テーブル見出しは必ずファイル末尾。** TOML は見出しから下がすべてその中身になるので、
  途中に置くとコメントを外した瞬間に後続のトップレベル項目が吸い込まれる
- ファイル中ほどにあった `[plugins.*]` の例は消した。同じ見出しの例が 2 つあると、
  上の方をコメント解除して壊す
- **罠: 説明文に ` = ` を書くと、機械的なコメント解除のテストが誤爆する**

## 罠

- **`cfg.top_level_patch_roles` を直接読んではいけない。** プロファイルの上書きも
  プラグインごとの組み込み既定も取りこぼす。しかも**それが効くのは既定プラグインだけ**。
  必ず `catalog_plugins()` / `PatchPlugins` を通すこと
- **焼き込んではいけない。** 焼き込むと土台が失われ、カタログに 2 つめのプラグインが載ったとき
  「そのプロファイルが書いていない項目」を解決できなくなる
- **残る穴（未対応）**: Surge をトップレベル `plugin_path` で標準以外の場所へ指定し、かつ
  `[plugins."Surge XT"]` にはカテゴリだけ書いた場合、そのプロファイルは実在チェック
  （組み込みの標準パスを見る）で落ちるため拾えない。末尾ブロックの `plugin_path` も
  一緒に書けば通る

## 壊れたら気づく場所

| テスト | 落ちたら |
|---|---|
| `patches/src/selection/tests.rs::free_keeps_every_patch_when_the_chord_categories_are_empty` | **Dexed の Free 行（chord mode off の全行）が候補 0 件になる** |
| `cmrt-runtime/src/plugin_profile/tests.rs::the_builtin_dexed_profile_does_not_narrow_the_patch_roles` | Dexed に Surge のカテゴリ名が復活する |
| `cmrt-runtime/src/plugin_profile/tests.rs::the_surge_profile_keeps_the_top_level_patch_categories` | 既存 Surge ユーザーの挙動が変わる |
| `cmrt-runtime/src/core_config/tests.rs::a_profile_for_the_default_plugin_still_contributes_its_patch_roles` | 末尾ブロックの編集案内が嘘になる |
| `tests::the_default_config_parses_with_and_without_the_commented_profile` | 末尾ブロックのコメント解除で config が壊れる |
| `config::tests::default_content::the_commented_profile_is_the_last_thing_in_the_default_config` | 同上 |
