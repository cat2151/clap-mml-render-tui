# ADR 0029: `cmrt.exe` と2つのサーバーの exe を静的リンクの zip にし、CI で毎日公開する

- 状態: 採用
- 決定者: user（cat-plugin-player・cat-smf-editor と同じ仕組みで毎日自動で release すること、release 本文は変える必要がある所以外は両 repo と同じにすること、用途は cat-plugin-player が cmrt の catalog を使えるようにすること）
- 関連: [0017](0017-play-server-binary-resolution.md)、[0023](0023-render-server-binary-resolution.md)、cat-plugin-player の ADR 0023（静的リンクの毎日 release）・ADR 0025（cmrt の catalog が無い環境）

## 決定

- `.github/workflows/release.yml` が毎日 01:23 JST に Windows x64 の zip を作り、`v<app/Cargo.toml の version>` の release を作り直す。
  build・CRT 検査・test がすべて成功したときだけ、既存の release と tag を消して作り直す。手動起動の既定はビルドと検査だけ。
- zip には `cmrt.exe`、`clap-mml-realtime-play-server.exe`、`clap-mml-render-server.exe`、`LICENSE`、`RUBBERBAND_LICENSE.ja.md`、`RUBBERBAND_COPYING` を入れる。
  `RUBBERBAND_COPYING` は、`rubberband-ffi/build.rs` が取得した commit の `COPYING`。
- サーバーの exe は、`Cargo.lock` が `clap-mml-play-server` に固定している commit からビルドする。
- 静的リンクは `-C target-feature=+crt-static` だけで切り替える。rubberband の C++ は cc が同じ flag を見て `/MT` を選ぶ。
  検査は cat-plugin-player の `scripts/crt_linkage.py` を CI で sparse checkout して使う。
- release 本文の原文は `.github/release-notes.md`。両 repo の原文から、次だけを変える。
  exe 名、uapmd の行を消すこと、Rubber Band が GPL であることの1行、commit 一覧（この repo と clap-mml-play-server の2つ）。
- 本文末尾の `<!-- build-inputs ... -->` が公開中の release と一致する日は、build と publish を飛ばす。

## 理由

- cat-plugin-player は、cmrt の catalog ファイルがあればそれを読み、無ければ Surge XT だけの catalog を読む（cat-plugin-player の ADR 0025）。
  cmrt の catalog は Surge XT 以外の CLAP 音色も持ち、3万件を超える（作者の環境で 35738 件）。
  `cmrt` の入手手段は `cargo install` だけだったので、この catalog を使えたのは Rust の環境を持つ人だけだった。
  release バイナリがあれば、Rust の環境が無い人も zip を展開するだけで catalog を作れて、cat-plugin-player の patch browser で3万件以上の音色を選べる。
- サーバーの exe を同梱するのは、`cmrt.exe` が同じディレクトリのサーバーを探すから（0017・0023）。PATH は見ない。
- `Cargo.lock` の commit を使うのは、`cmrt.exe` に組み込まれる `cmrt-core` とサーバーの exe を同じ commit にするため。
  `Cargo.lock` は 01:00 JST の `update-cat2151-rust-deps.yml` が main の HEAD へ上げるので、ふだんは最新になる。
- Rubber Band を静的リンクした `cmrt.exe` を配るので、`RUBBERBAND_LICENSE.ja.md` の「バイナリを配布する場合」に従い、取得した版の `COPYING` を添える。
- cat-smf-editor の共有 release エンジンは、uapmd と CMake を前提にしていて、ルートに `[package]` がある構成を想定している。
  この repo はどれにも当たらないので、cat-plugin-player の単一 workflow の形を使う。
- 罠: `cargo test` は dev-dependencies の `test-support` feature を付けて bin をビルドし直す。だから zip は test より先に作る。

## 却下した案

| 案 | 理由 |
| --- | --- |
| サーバーを clap-mml-play-server の main の HEAD からビルドする | `cmrt.exe` に組み込まれる `cmrt-core` と commit がずれる日が出る |
| `cmrt.exe` だけを配り、サーバーは別に入れてもらう | 同じディレクトリに無いとサーバーが見つからず、音が出ない |
| `crt_linkage.py` をこの repo へ写す | 同じ検査が2か所に分かれる。exe に入らない道具なので、build-inputs に入れなくてよい |
| cat-smf-editor の共有エンジンをこの repo 向けに書き換えて使う | エンジンを変えずに共有 repo へ移す、という前提が崩れる |

## 結果

- zip を展開したディレクトリで `cmrt` を起動すると、サーバーは `同梱` の profile で決まり、画面に警告は出ない（0017）。
- サーバー側だけを直した日は、`Cargo.lock` が上がるまで zip に入らない。
- テストで通るのは自動の検査だけで、配布物を人が動かして確かめてはいない。release 本文にもそう書く。
