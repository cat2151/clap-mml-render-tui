# Rubber Band の利用条件とソース取得方法

このプロジェクトは、Track List の WAV タイムストレッチに
[Rubber Band Library](https://github.com/breakfastquay/rubberband) を使用します。

## ライセンス

Rubber Band Library は Particular Programs Ltd.（Breakfast Quay）により、
**GNU General Public License version 2 または、それ以降の任意のバージョン
（GPL-2.0-or-later）**で提供されています。別途、権利者から商用ライセンスを
取得した場合は、その契約条件を適用できます。

- Rubber Band copyright: Copyright 2007-2024 Particular Programs Ltd.
- Rubber Band 公式ライセンス: <https://breakfastquay.com/rubberband/license.html>
- GPL version 2 本文: <https://www.gnu.org/licenses/old-licenses/gpl-2.0.txt>

ルートの `LICENSE` に記載された MIT License は、このプロジェクトが独自に
作成したソースファイルに適用されます。Rubber Band を静的リンクした実行形式を
配布する場合、その結合された成果物は Rubber Band の GPL 条件を満たす形で
配布する必要があります。この文書は法的助言ではありません。配布者は自身の
配布形態についてライセンス条件を確認してください。

## Cargo ビルド時の取得

Rubber Band のソース一式は、この Git リポジトリへ commit/push しません。
`cargo build`、`cargo test`、`cargo clippy` などの Cargo コマンドが
`rubberband-ffi/build.rs` を実行し、次の GitHub リポジトリから Cargo の
ビルド出力領域へ一時的に clone/fetch します。

```text
https://github.com/breakfastquay/rubberband
```

上流の既定ブランチ名は `default` です。再現可能なビルドにするため、
ビルドスクリプトは `rubberband-ffi/build.rs` の `UPSTREAM_REVISION` に記載した
commit を取得し、公式の
`single/RubberBandSingle.cpp` を built-in FFT / built-in resampler 構成で
コンパイルします。取得した Rubber Band ソースや生成したネイティブライブラリは
Cargo の `target/` 以下だけに置かれ、Git 管理対象にはなりません。

使用する commit を更新するときは、`UPSTREAM_REVISION` を `origin/default` の
更新時点の HEAD へ変更します。通常の Cargo 起動では上流 HEAD を問い合わせないため、
Rubber Band に関係しない連続ビルドで C++ ライブラリを再コンパイルしません。

ネットワーク、Git、または対応する C++ toolchain を利用できない場合、ビルドは
具体的なエラーを表示して失敗します。Rubber Band を使わない代替ビルドはありません。

## 使用したソースの特定と取得

ビルド済み `cmrt` が使用している Rubber Band の C API major version と Git
commit は、次のコマンドで確認できます。

```console
cmrt --version
```

表示された Rubber Band commit を `REVISION` とすると、対応ソースは次のように
取得できます。

```console
git clone https://github.com/breakfastquay/rubberband
cd rubberband
git checkout REVISION
```

または GitHub の次の URL から同じ commit のアーカイブを取得できます。

```text
https://github.com/breakfastquay/rubberband/archive/REVISION.tar.gz
```

取得したソースに含まれる `COPYING` が、そのビルドで使われた Rubber Band の
GPL version 2 本文です。`single/RubberBandSingle.cpp` は built-in FFT と
built-in resampler を使用するため、FFTW、Intel IPP、KissFFT、Speex、
libsamplerate はこの構成ではリンクしません。

## バイナリを配布する場合

配布者は少なくとも次を確認してください。

1. `cmrt` と Rubber Band を含む結合作品を GPL 条件に従って配布する。
2. このプロジェクトのビルドに使った完全な対応ソース、ビルドスクリプト、
   Rubber Band の正確な commit の対応ソースを GPL の定める方法で提供する。
3. Rubber Band ソースの `COPYING` と著作権・無保証表示を成果物へ添付する。
4. 受領者による改変・再配布へ GPL 以上の追加制限を課さない。
5. GPL と両立しない配布条件が必要な場合は、配布前に Breakfast Quay から
   商用ライセンスを取得する。

GPL の正確な条件は、必ず取得した Rubber Band ソースの `COPYING` を参照して
ください。
