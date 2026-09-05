# ADR — 設計判断の記録

**確定した設計判断とその理由**を残す。
「なぜそうしなかったのか」が残っていないと将来復元できないものだけを置いている。

前半（0001〜0015）は CLAP プラグイン抽象化と Surge XT / Dexed / Vaporizer2 / Sforzando の混在、
後半（0016〜0019）は DAW の live 演奏と daily キャッシュ。
利用者向けの現行仕様は `README.ja.md` にある。

ここに載っている実装はすべて完了済み。

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
| [0012](0012-live-clock-drift-is-absorbed-not-eliminated.md) | live クロックの先行は、消さずに吸収する |
| [0013](0013-server-owned-audio-plugin-abstraction.md) | オーディオプラグインの具象知識は server 側へ置く |
| [0014](0014-fixed-surge-primary-plugin.md) | config の既定プラグインは Surge XT に固定する |
| [0015](0015-sforzando-shared-catalog.md) | Sforzando は loadable program だけを共有 catalog へ載せる |
| [0016](0016-daw-live-playback-slots-and-timeline.md) | DAW の live 演奏はスロットと timeline で鳴らす |
| [0017](0017-play-server-binary-resolution.md) | play server の実体は探索順で決める（PATH 解決は廃止） |
| [0018](0018-page-replacement-clears-the-cache.md) | ページを全置換する経路はキャッシュ WAV を掃除する |
| [0019](0019-investigation-stage-acceptance.md) | 調査 Stage は受け入れ条件の外に主張を置かない |

## 未解決として残している論点

**patch → プラグインの判別規則を「拡張子で決める」ままにするか**（[0001](0001-patch-string-decides-the-plugin.md)）。
踏み込むと display 文字列＝永続 ID が変わり、保存済みデータの移行が発生するため、いま踏み込まない。

3 つめのプラグイン（Vaporizer2 / `.vvp`）はこの論点に触れたが、
**固有拡張子があったので永続 ID を変えずに解けた**（`PatchForm` を 3 値へ広げただけ）。
残っているのは「**同じ拡張子を扱うプラグインが 2 つ載る**」場合で、そのときは
patch 文字列の形を変えるか、config で dir ごとにプラグインを明示するかの二択になる。

**DAW の演奏で残している穴**は [0018](0018-page-replacement-clears-the-cache.md) の
「塞いでいない穴」の表にまとめてある（Persistent 側の同型の穴 / WAV の長さ検証 /
書き込み中の WAV / ループ長 5・9・13 のスロット余裕 0）。
どれも「前日の音が混ざる」「モタる」「ぶつ切り」の説明ではない、独立した挙動。
