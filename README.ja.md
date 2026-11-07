# clap-mml-render-tui

## 概要
MML TUI DAW（のようなもの）。[Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) のリッチな音を、MMLで手軽に楽しめます。Rustで書かれています。

## 用途

- MMLで音を鳴らして遊ぶ用
- 自分用。実験プロジェクト

## 技術スタック
- プラグインホスト用ライブラリ
  - https://github.com/prokopyl/clack

## 準備

[Surge XT](https://surge-synthesizer.github.io/)をinstallしてください

```
winget install "Surge XT"
```

※検討中 : 上記の自動化（ cat-plugin-player で実現済み）

## install

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

※検討中 : 自動release

## 実行

```
cmrt
```

TUI画面でMMLを入力して遊べます

## 音色catalog

Dexedなどの音色catalogを作成し、絞り込みなどに活用できます

```
cmrt build-patch-catalog-cache
```

## 対応オーディオプラグイン
- ※CLAP、Windows、アカウント登録なしで無料で入手できるもの、に絞っています
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- effect（DAWモード の track に直列で挿せます）
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects（Surge XTに同梱）
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
- sfz（のうち、専用画面でキースイッチとコントロールチェンジを試せるもの）
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- それぞれのダウンロード画面（迷子になった人向け）
  - Surge XT : wingetで入手が楽です : [最速！猫でもインストールできるDAWとオーディオプラグイン（仮想MIDI鍵盤で音を鳴らすまで）](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed（studiorack-siteでの紹介ページ）](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2（studiorack-siteでの紹介ページ）](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe ダウンロード画面](https://floe.audio/download/)
  - [Sforzando ダウンロード画面](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub releases画面](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 ダウンロード画面](https://u-he.com/products/tyrelln6/)
  - [TONE3000 ダウンロード画面](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub releases画面](https://github.com/michaelwillis/dragonfly-reverb/releases)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 

## update

```
cmrt update
```

## SNSの投稿を音楽として鳴らす

```
cmrt --server
```

詳しくは、 [bluesky-text-to-audio](https://github.com/cat2151/bluesky-text-to-audio/blob/main/README.ja.md) を参照ください

## 破壊的変更
- 毎日頻繁に破壊的変更します。昨日鳴っていたものが今日は鳴らない、というのは日常茶飯事です

## コンセプトのメモ
- アトミック小節
    - Obsidianのアトミックノートに着想を得たものです。
    - あらゆる処理の単位を、「1小節単位のオフラインレンダリング」にすることで、
    - 制約を受けるかわりに、
    - いろいろなメリットを獲得できます。
    - これはスケッチ用途、素早く編集のサイクルをまわす用途に向きます。
    - より本格的な編集が必要なら、既存の高機能なDAWのほうが向くでしょう。
    - ※atomic measure だと物理学の言葉になってしまうので、ひとまず英訳せず「アトミック小節」のままにしておきます。
- 実験
    - いろいろな実験を実装します
- 自分専用
    - 破壊的変更を頻繁に行います
        - 昨日saveしたファイルが今日loadできないのは日常茶飯事となります
            - なので利用はおもちゃレベルにとどめてください

## スコープ外
- 実用性

## ライセンス
- MIT
