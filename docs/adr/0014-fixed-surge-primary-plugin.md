# ADR 0014: config の既定プラグインを Surge XT に固定する

- 状態: 採用（2026-08-24）
- 置換: [0002](0002-config-plugin-profiles.md) の `active_plugin` 選択方式
- 関連: [0004](0004-default-plugin-owns-unspecified-patches.md) / [0005](0005-mixed-catalog-on-by-default.md) / [0007](0007-patch-role-defaults-three-layers.md)

## 決定

音色を指定していない行が鳴る既定プラグインは Surge XT に固定する。
config.toml に選択状態は保存せず、`active_plugin` は値にかかわらず設定エラーにする。

plugin 固有の設定は `[plugins.<名前>]` の中だけに置く。
Surge XT の標準値を変える場合は `[plugins."Surge XT"]` に差分を書く。
名前照合は既存どおり大文字小文字・空白・underscore の違いを吸収するので、
`[plugins.SurgeXT]` も同じ override として扱う。

次のキーが TOML のトップレベルにあれば、未知キーとして黙って無視せず設定エラーにする。

- `active_plugin`
- `plugin_path`, `plugin_id`, `patches_dirs`
- `chord_patch_categories`, `bass_patch_categories`, `arpeggio_patch_categories`, `drum_patch_categories`
- `kick_patch_keywords`, `snare_patch_keywords`, `hihat_patch_keywords`

Dexed、Vaporizer2、Floe、Sforzando など Surge XT 以外の profile は削除しない。
これらは既定を切り替える設定ではなく、既存の混在 patch catalog へ載せる候補である。

## 内部境界

config の公開構文からトップレベル plugin 設定を削除しても、`Config` の
`plugin_path` / `plugin_id` / `patches_dirs` と用途別 role 差分は、解決済みの
runtime view として残す。ロード時に組み込み Surge XT と `[plugins."Surge XT"]` を merge し、
その結果を既存 field へ焼き込む。

この境界により、patch routing、catalog の先頭要素、cache 名前空間、render/play server への
受け渡しは変更しない。config の構文変更を理由に下流 caller を一斉改修しない。

解決と旧トップレベルキー検査の単一ソースは sibling play-server repo の
`cmrt-server-config` とする。TUI の `Config` と server の `ServerConfig` は同じ helper を通す。

## 移行

旧 config は自動推測で救済しない。ユーザー管理の config をバックアップしたうえで、
`active_plugin` を削除し、トップレベルの plugin 固有値を `[plugins."Surge XT"]` へ移す。
複数の旧キーを検出した場合は、1 回のエラーに全キー名と移動先を表示する。

## 採らなかった案

- `active_plugin = "Surge XT"` だけを互換目的で受理する: 廃止済み状態を残し、再び選択機能に見える
- runtime field まで直ちに削除する: 多数の caller と server 境界を同時に変え、回帰範囲が広がる
- 他 plugin profile も削除する: 混在 catalog と patch ごとの routing まで別仕様にしてしまう

## 回帰を検出する条件

- `active_plugin` と旧トップレベルキーは TUI / server の実ロード経路で失敗する
- `[plugins."Surge XT"]` の差分だけが固定既定へ焼き込まれる
- 他 profile を追加しても既定は Surge XT のまま、混在 catalog には残る
- 新規生成 config は旧トップレベルキーを値として出力しない
