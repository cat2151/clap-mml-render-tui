# ADR 0006: display 文字列はプロファイルごとの base で相対化する

- 状態: 採用
- 関連: [0001](0001-patch-string-decides-the-plugin.md) / [0005](0005-mixed-catalog-on-by-default.md)

## 決定

`collect_patch_pairs` は「全音色ディレクトリの共通親 1 本」を base にするのをやめ、
**カタログのプラグインごとに base を選んで相対化し、その結果を連結する。**

```
Surge XT: base = C:\ProgramData\Surge XT
          → display = patches_factory/...  /  patches_3rdparty/...
Dexed   : base = %APPDATA%\DigitalSuburban\Dexed\Cartridges
          → display = Dexed_01.syx/00 Say Again.
Sforzando: base = <設定した 2 ルートの共通親>
          → display = Plogue/Free Sounds/Programs/.../*.sfz / sfz/.../*.sfz
```

## 理由: 素朴に union すると壊れる

```
shared_patch_root_dir([
  C:\ProgramData\Surge XT\patches_factory,
  C:\ProgramData\Surge XT\patches_3rdparty,
  C:\Users\<user>\AppData\Roaming\DigitalSuburban\Dexed\Cartridges,
]) == "C:\"
```

display が `ProgramData/Surge XT/patches_factory/...` と
`Users/<user>/AppData/Roaming/DigitalSuburban/Dexed/Cartridges/Dexed_01.syx/...` になる。結果:

- category が `ProgramData` / `Users` になり、**用途別絞り込みが全滅する**
- **display 文字列は永続 ID なので、保存済みの MML / history / DAW セル / grid session が
  全部指し先を失う**

プロファイルごとの base なら **display は今日とビット単位で同一**になる。
**後方互換が完全に保たれるのが決め手。**

## 先頭コンポーネントが既にプラグインの識別子になっている

| プラグイン | display の形 | 先頭コンポーネント |
|---|---|---|
| Surge XT | `patches_factory/<category>/<patch>.fxp` | `patches_factory` |
| Surge XT | `patches_3rdparty/<vendor>/<category>/<patch>.fxp` | `patches_3rdparty` |
| Dexed | `Dexed_01.syx/00 Say Again.` | cartridge ファイル名 |
| Sforzando | `sfz/<library>/<patch>.sfz` | 設定ルートを区別する先頭ディレクトリ |

カテゴリ抽出（play-server `core-lib/src/audio_plugin.rs` の `patch_sort_metadata()`）は
prefix を strip して次のセグメントを category にし、
**どちらの prefix でもなければ先頭ディレクトリを category にする**。
Dexed はこの枝を通り `Dexed_01.syx` が category になる。
**したがって混在カタログにしてもカテゴリ分けは壊れない。**

## crate の分け方

`cmrt-patches`（`patches/`）はプラグイン中立。path の形をプラグインごとに読む知識は
play-server の shared core（`patch_sort_metadata()`）が持ち、この crate はその結果だけを扱う
（`layout.rs` がその facade）。

**prefix の無い path は、cartridge 名も prefix 抜きで保存された Surge の名前も shared core の
同じ枝に落ちる。** これは意図的で、どちらも先頭セグメントをカテゴリとして読み供給元の優先度も
0 なので結果が変わらない。

## 壊れたら気づく場所

- `patches/src/layout/tests.rs::abstract_metadata_keeps_prefixless_categories`
  — この同値が崩れると保存済みの patch 名がカテゴリを失う
- `tui-core/src/patches/tests.rs` — カタログにプラグインが増えても既存の音色の指し先が変わらないこと
