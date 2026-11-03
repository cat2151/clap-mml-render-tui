# ADR 0030: patch selector の2つ目の欄は、画面では `Type` と呼び、保存形式とコードは `preset` のままにする

- 状態: 採用
- 決定者: user（cmrt の画面も cat-plugin-player と同じ `Type` に揃えること）。保存形式とコードの名前を変えないことは、cat-plugin-player で user が採用した AI の提案を cmrt にも当てたもの
- 関連: [0028](0028-selector-list-follows-preset-order.md) / [cat-plugin-player ADR 0042](../../../cat-plugin-player/docs/adr/0042-type-pane-keeps-preset-key.md)

## 決定

- Role 欄の隣の欄（strings / pad / kick / ★ Favorite / Drum kit / user の Regex 条件など）を、画面では `Type` と表示する。対象は、この欄を出す画面すべて（notepad・mml overlay の音色選択、grid sequencer の patch selector、keyboard 画面）と、ヘルプ・ステータス行・保存失敗の表示。
- コードの識別子（`FilterPreset` / `preset_rank` / `PatchPaneFocus::Preset` など）と、user の Regex 条件の保存ファイル（`mml_patch_filters.json` の `presets`）は変えない。データ移行もしない。

## 理由

- `Preset` は plugin のプリセット（音色そのもの）と紛らわしい。cmrt には effect chain の preset 一覧もあり、同じ語が別の物を指す。
- `Category` は、同じ画面の音色一覧にある Category 列（plugin 側の分類）と衝突する。
- cat-plugin-player は既にこの欄を `Type` と表示している。同じ欄の名前が画面ごとに違うと、両方を使うときに迷う。
- 保存ファイルは cat-plugin-player と共有している。キーを変えると、片方だけ新しいビルドのときに相手が書いた条件を読めない。

## 却下した案

- **コードと保存キーも `type` に揃える。** 画面上の得は無く、共有する保存ファイルの移行と、両 repo の同時更新が要る。
- **cmrt の画面は `Preset` のまま残す。** cat-plugin-player と同じ欄の名前が食い違う。

## 結果

- 画面は Type、コードと保存形式は preset と、呼び名が2つ並ぶ。
- ADR 0028 などの既存の ADR は、この欄を「Preset 欄」と書いたまま残す。
