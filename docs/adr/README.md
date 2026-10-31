# ADR — 設計判断の記録

**確定した設計判断とその理由**を残す。
「なぜそうしなかったのか」が残っていないと将来復元できないものだけを置いている。

0001〜0015 は CLAP プラグイン抽象化と Surge XT / Dexed / Vaporizer2 / Sforzando の混在、
0016〜0019 は DAW の live 演奏と daily キャッシュ、0020〜0021 は chord chart 画面と auto voicing、
0022 は DAW の effect chain、0023〜0024 は offline render の経路（render-server の実体と 1 経路化）、
0025 は Guitar Articulation 画面で METAL-GTX の効かない操作を出さないこと、0026 はそのアルペジエーター overlay とメイン画面の線引き。
0027 は先頭 JSON 付き MML を「;」でトラックに分けた形を扱わないこと。
0028 は patch selector の一覧をどの Role でも Preset 欄の順に並べること。
0029 は `cmrt.exe` と2つのサーバーの exe を静的リンクの zip にし、CI で毎日公開すること。
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
| [0009](0009-offline-entry-map.md) | オフラインレンダリングは MML ごとに entry を引き分ける（[0024](0024-offline-render-goes-through-the-render-server-only.md) で廃止） |
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
| [0020](0020-chord-chart-does-not-interpret-strings.md) | chord chart 画面は文字列を解釈しない |
| [0021](0021-bass-may-double-the-lowest-chord-note.md) | auto voicing の Bass は chord layer の最低音と unison になってよい |
| [0022](0022-effect-chain-in-init-json.md) | effect chain は init JSON の `"effects after instrument"` に持ち、TUI は解釈しない |
| [0023](0023-render-server-binary-resolution.md) | render-server の実体も play server と同じ順で探し、PATH を見ない |
| [0024](0024-offline-render-goes-through-the-render-server-only.md) | offline render は render-server の 1 経路だけ。`cmrt.exe` は CLAP をロードしない |
| [0025](0025-metal-gtx-unused-controls.md) | METAL-GTX の Stop_Key・「C0 を短く」・CC31・トリルへの CC28 は効かないので画面に出さない |
| [0026](0026-arp-overlay-owns-what-to-play.md) | アルペジエーター overlay は素材とアルペジエーターの設定を自分で持ち、行全体のルール・アクセントはメイン画面と共有する |
| [0027](0027-leading-json-with-semicolon-tracks-is-out-of-scope.md) | 先頭 JSON 付き MML を「;」でトラックに分けた形は扱わない（運用データに無いので、対応も報告もしない） |
| [0028](0028-selector-list-follows-preset-order.md) | patch selector の一覧は、どの Role でも Preset 欄の順に並べる |
| [0029](0029-daily-static-crt-release.md) | `cmrt.exe` と2つのサーバーの exe を静的リンクの zip にし、CI で毎日公開する |
