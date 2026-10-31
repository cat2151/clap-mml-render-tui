$name v$version (Windows x64)

## 配布物
- `$name-$version-windows-x64.zip`: `cmrt.exe` と $package_files。VC++ ランタイムを静的リンクしているので、Microsoft Visual C++ 再頒布可能パッケージは不要
- Rubber Band Library（GPL-2.0-or-later）を静的リンクしているので、`cmrt.exe` は GPL の条件で配布している。条件とソースの取得方法は同梱の `RUBBERBAND_LICENSE.ja.md`

## この版について
- GitHub Actions が毎日自動でビルドし、公開している（下の commit がどれも変わらない日は作り直さない）
- 通っているのは自動の検査（ビルド、VC++ ランタイムのリンク検査、テスト）だけで、人による動作確認はしていない

<details>
<summary>ビルドに使った commit</summary>

$commits

</details>

$build_inputs
