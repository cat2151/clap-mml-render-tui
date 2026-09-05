# ADR 0015: Sforzando は loadable program だけを共有 catalog へ載せる

- 状態: 採用（2026-08-23、state adapter の実機確認後に訂正）
- 関連: [0001](0001-patch-string-decides-the-plugin.md) / [0005](0005-mixed-catalog-on-by-default.md) /
  [0006](0006-per-profile-relative-base.md) / play-server `docs/adr/0015-sforzando-sfz-preset-load.md`

## 決定

- builtin `Sforzando` profile を既存の画面横断 catalog へ載せる
- TUI は ARIA registry、bank ID/version、`*.bank.xml`、CEGP state を知らない。play-server の
  plugin-neutral な patch-source resolver から `dirs`, `resolved_patches`, diagnostics だけを受け取る
- `resolved_patches` がある plugin は directory を再走査せず、その canonical file list だけを表示する
- Sforzando adapter は registry `user_files_dir` の user bank と、configured root 周辺の installed bank
  manifest を解決する。canonical path が検証済み program に対応する SFZ だけを返す
- display value は plugin ごとの共有 base から相対化した従来の `.sfz` path。MML の
  `"Surge XT patch"`、history、DAW/grid session、realtime wire は変更しない

`.sfz` という patch form の routing は共有ドメイン知識として TUI に残る。ARIA program 座標と state
構築は表示責務ではないため play-server 内に閉じ込める。

## preset-discovery を source にしない理由

Sforzando 2.1.2.4 の provider は filesystem `.sfz` location ではなく PLUGIN location `factory` を 1 件返す。
factory preset は load key でロードできるが、任意 SFZ の FILE preset-load は `false` だった。したがって
factory discovery と user/installed SFZ catalog は別機能として扱う。

## 実測 catalog

この環境では user bank 529 件 + Free Sounds manifest 登録 54 件 = **583 件**。Free Sounds directory に
実在する残り 12 件（CR-909 component 11 件と未登録 Xylophone）は選べても鳴らせないため除外する。
除外件数と source の部分的な破損は `source_notices` と `patch-load: event=source-notice` に残し、使える
source は catalog に維持する。source が全く解決できなければ generic な `PatchSourceUnavailable` とする。

`source_notices` は catalog library から stdout/stderr へ直接書かない。TUI の alternate screen 中に標準
stream へ書くと ratatui の差分描画が壊れるため、画面には既存の catalog note として渡し、永続ログは app が
注入した sink から `log/log.txt` へ書く。子 server は standalone 実行時には stderr を診断 stream として
使えるが、app が起動した realtime/render server の stderr は supervisor が pipe し、同じ log sink へ転送する。
play-server の共有 core も同じ契約とし、未注入（server process）では stderr、TUI process では app が注入した
非同期 sink を使う。これにより catalog 以外の CLAP / patch 診断も alternate screen へ漏らさない。

## 番人

- play-server `server-config/src/sforzando_programs/tests.rs::installed_catalog_contains_583_loadable_programs`
- `tui-core/src/patches/tests.rs::adapter_resolved_paths_are_used_without_rescanning_vendor_files`
- `tui-core/src/patch_plugins/tests.rs::five_plugin_catalog_routes_sfz_only_to_sforzando`
- `app/src/tui/tests/sforzando_screens.rs`
- `cmrt-runtime/src/core_config/tests.rs::adapter_reports_config_and_program_source_failures_together`
