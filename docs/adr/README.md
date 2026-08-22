# ADR — 設計判断の記録

CLAP プラグイン抽象化と Surge XT / Dexed / Vaporizer2 混在に関する、**確定した設計判断とその理由**。
「なぜそうしなかったのか」が残っていないと将来復元できないものを残している。

実装はすべて完了済み。利用者向けの現行仕様は `README.ja.md`「複数プラグインの使い分け」節にある。

play-server 側（プラグインの実測仕様・実行時の設計）は
`../clap-mml-play-server/docs/adr/` にある。依存の向きが TUI → play-server の一方向
（[0010](0010-two-repo-layout.md)）なので、ADR も repo ごとに閉じている。

| # | 決定 |
|---|---|
| [0001](0001-patch-string-decides-the-plugin.md) | patch 文字列がプラグインを決める |
| [0002](0002-config-plugin-profiles.md) | config はプラグインプロファイル方式 |
| [0003](0003-mml-patch-key.md) | MML 先頭 JSON のキーは `"Surge XT patch"` を流用する |
| [0004](0004-default-plugin-owns-unspecified-patches.md) | 音色を無指定にした行は既定プラグインで鳴る / cache 名前空間 |
| [0005](0005-mixed-catalog-on-by-default.md) | 混在カタログは既定 ON。実在する dir だけ載せる |
| [0006](0006-per-profile-relative-base.md) | display 文字列はプロファイルごとの base で相対化する |
| [0007](0007-patch-role-defaults-three-layers.md) | 用途別カテゴリの既定値は 3 層で解決する |
| [0008](0008-voicing-per-patch.md) | voicing は patch ごとに引く / 未知プラグインは poly とみなす |
| [0009](0009-offline-entry-map.md) | オフラインレンダリングは MML ごとに entry を引き分ける |
| [0010](0010-two-repo-layout.md) | 2 repo 構成は TUI → play-server の一方向 |
| [0011](0011-verification-and-baselines.md) | 検証手段と実測ベースライン |

## 未解決として残している論点

**patch → プラグインの判別規則を「拡張子で決める」ままにするか**（[0001](0001-patch-string-decides-the-plugin.md)）。
踏み込むと display 文字列＝永続 ID が変わり、保存済みデータの移行が発生するため、いま踏み込まない。

3 つめのプラグイン（Vaporizer2 / `.vvp`）はこの論点に触れたが、
**固有拡張子があったので永続 ID を変えずに解けた**（`PatchForm` を 3 値へ広げただけ）。
残っているのは「**同じ拡張子を扱うプラグインが 2 つ載る**」場合で、そのときは
patch 文字列の形を変えるか、config で dir ごとにプラグインを明示するかの二択になる。
