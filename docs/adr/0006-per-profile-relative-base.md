# ADR 0006: display 文字列はプロファイルごとの base で相対化する

- 状態: 採用（2026-08-20）
- 関連: [0001](0001-patch-string-decides-the-plugin.md) / [0005](0005-mixed-catalog-on-by-default.md)

## 決定

`collect_patch_pairs` は「全音色ディレクトリの共通親 1 本」を base にするのをやめ、
**カタログのプラグインごとに base を選んで相対化し、その結果を連結する。**

```
Surge XT: base = C:\ProgramData\Surge XT
          → display = patches_factory/...  /  patches_3rdparty/...
Dexed   : base = %APPDATA%\DigitalSuburban\Dexed\Cartridges
          → display = Dexed_01.syx/00 Say Again.
```

## 理由: 素朴に union すると壊れる

```
shared_patch_root_dir([
  C:\ProgramData\Surge XT\patches_factory,
  C:\ProgramData\Surge XT\patches_3rdparty,
  C:\Users\f\AppData\Roaming\DigitalSuburban\Dexed\Cartridges,
]) == "C:\"
```

display が `ProgramData/Surge XT/patches_factory/...` と
`Users/f/AppData/Roaming/DigitalSuburban/Dexed/Cartridges/Dexed_01.syx/...` になる。結果:

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

カテゴリ抽出（`patches/src/layout.rs` の `patch_category_sort_parts()`）は最初から両形式を
1 本で扱っている: prefix を strip して次のセグメントを category にし、
**どちらの prefix でもなければ先頭ディレクトリを category にする**。
Dexed は今すでにこの else 節を通り、`Dexed_01.syx` が category になっている。
**したがって混在カタログにしてもカテゴリ分けは壊れない。**

## crate の分け方

`cmrt-patches`（`patches/`）はプラグイン中立。Surge の知識は 1 module に隔離してある。

| module | 中身 |
|---|---|
| `patches/src/surge_xt.rs` + `surge_xt/{layout,defaults}.rs` | `patches_factory` / `patches_3rdparty` の prefix 解析と、用途別の既定カテゴリ名。**Surge の知識はここだけ** |
| `patches/src/cartridge.rs` | `<cartridge>.syx/<voice>` の 1 階層。カートリッジ名がカテゴリ、供給元の優先度は常に 0 |
| `patches/src/layout.rs` | 中立の入口。`PatchLayout::of(path)` が形から体系を選ぶ |

**`PatchLayout::Cartridge` には prefix 抜きで保存された Surge の名前も落ちる。** これは意図的で、
どちらも先頭セグメントをカテゴリとして読み供給元の優先度も 0 なので結果が変わらない。

## 壊れたら気づく場所

- `patches/src/layout/tests.rs::a_prefixless_surge_name_reads_the_same_either_way`
  — この同値が崩れると保存済みの patch 名がカテゴリを失う
- `tui-core/src/patches/tests.rs` — カタログにプラグインが増えても既存の音色の指し先が変わらないこと
