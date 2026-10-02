# ADR 0015: Sforzando は loadable program だけを共有 catalog へ載せる

- 状態: 採用
- 関連: [0001](0001-patch-string-decides-the-plugin.md) / [0005](0005-mixed-catalog-on-by-default.md) /
  [0006](0006-per-profile-relative-base.md) / play-server `docs/adr/0015-sforzando-sfz-preset-load.md`

## 決定

- builtin `Sforzando` profile を既存の画面横断 catalog へ載せる
- TUI は ARIA registry、bank ID/version、`*.bank.xml`、CEGP state を知らない。play-server の
  plugin-neutral な patch-source resolver から `dirs`, `resolved_patches`, diagnostics だけを受け取る
- `resolved_patches` がある plugin は directory を再走査せず、その canonical file list だけを表示する
- Sforzando adapter の走査 root は ARIA の registry だけで決める。`user_files_dir` の user bank と、
  `Aria\Products` から辿る installed bank（`bank_path` の `*.bank.xml` のあるディレクトリ）。
  `[plugins.Sforzando] patches_dirs` は使わない。state の `Slot` は bank 座標しか持たず、ARIA に
  登録されていない場所の SFZ は一覧に出せても鳴らせないため。canonical path が検証済み program に
  対応する SFZ だけを返す
- display value は音色置き場ごとの親から相対化した `.sfz` path（先頭要素が置き場のフォルダ名。
  [0006](0006-per-profile-relative-base.md) の「Sforzando は置き場ごと」）。基点は resolver が
  `PatchCatalogResolution.base` で返し、TUI は plugin 名で分岐しない

`.sfz` という patch form の routing は共有ドメイン知識として TUI に残る。ARIA program 座標と state
構築は表示責務ではないため play-server 内に閉じ込める。

## preset-discovery を source にしない理由

Sforzando 2.1.2.4 の provider は filesystem `.sfz` location ではなく PLUGIN location `factory` を 1 件返す。
factory preset は load key でロードできるが、任意 SFZ の FILE preset-load は `false` だった。したがって
factory discovery と user/installed SFZ catalog は別機能として扱う。

## 実測 catalog

開発環境では installed bank が Free Sounds 54 件 + TableWarp2 1 件 = **55 件**（番人テストの期待値）。
user bank は `.sfz` を置くだけで増えるので、番人テストはその場で数えた `.sfz` の件数と比べる。
manifest に無い `.sfz` は directory に実在しても選べるだけで鳴らせないため除外する。
除外件数と source の部分的な破損は `source_notices` と `patch-load: event=source-notice` に残し、使える
source は catalog に維持する。source が全く解決できなければ generic な `PatchSourceUnavailable` とする。

`source_notices` は catalog library から stdout/stderr へ直接書かない。TUI の alternate screen 中に標準
stream へ書くと ratatui の差分描画が壊れるため、画面には既存の catalog note として渡し、永続ログは app が
注入した sink から `log/log.txt` へ書く。play-server の共有 core も同じ契約で、未注入（standalone の server
process）では stderr、TUI process では app が注入した非同期 sink を使う。app が起動した realtime/render server
の stderr は supervisor が pipe して同じ sink へ転送するので、catalog 以外の CLAP / patch 診断も alternate screen へ漏れない。

## 番人テスト

- play-server `plugin-presets/src/sforzando/tests/installed.rs::installed_catalog_explains_every_unlisted_user_program`
- `tui-core/src/patches/tests.rs::adapter_resolved_paths_are_used_without_rescanning_vendor_files`
- `tui-core/src/patch_plugins/tests.rs::five_plugin_catalog_routes_sfz_only_to_sforzando`
- `app/src/tui/tests/sforzando_screens.rs`
- `app/src/patch_catalog_cache/tests/installed_sforzando.rs`（ignored、`CMRT_TEST_SFORZANDO_CLAP`）
  — 実機の全 Sforzando display が置き場のフォルダ名で始まり、実在ファイルへ解決される
- `cmrt-runtime/src/core_config/tests.rs::adapter_reports_config_and_program_source_failures_together`
