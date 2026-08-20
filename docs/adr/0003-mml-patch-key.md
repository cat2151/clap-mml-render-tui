# ADR 0003: MML 先頭 JSON のキーは `"Surge XT patch"` を流用する

- 状態: 採用（2026-08-20）
- 関連: [0001](0001-patch-string-decides-the-plugin.md) / [0004](0004-default-plugin-owns-unspecified-patches.md)

## 決定

```json
{"Surge XT patch": "SynprezFM/SynprezFM_01.syx/01 Say Again."} t120 o4 cde
```

キー名は変えない。**値の形（`.syx` を含むか）でプラグインが分かる**ので、
キーに種別を持たせる必要がない。

## 理由

読み書きコードは変更なし。過去の MML / history / DAW 保存データもそのまま動く。
Dexed 対応の初期に「1 プロセス 1 プラグインだからキーは実質『今のプラグインの音色』を意味する」
という理由で流用したが、**混在を patch 文字列駆動（[0001](0001-patch-string-decides-the-plugin.md)）に
した結果、混在後もこの流用が成立したまま**になった。

## 帰結: IPC / SHM / 永続データは無改修

patch 文字列そのものがプラグインを決めるなら、IPC に足す情報は 0。したがって以下はすべて不要:

- **SHM の VERSION 上げは不要**（新しいフィールドを足さないので古いサーバーとの不整合が起きない）
- **`"CLAP preset"` JSON wire 形式は不要**
- **`PresetRef` tagged enum は不要**
- **DAW / history / notepad / mml-overlay / grid session の永続データは無改修**

## 承知している難点

Dexed 使用時にキー名が実態と合わない。表示上の違和感だけで、動作には影響しない。

## 壊れたら気づく場所

- `core-lib/src/tests.rs` の `embedded_patch_ref` 系 — 先頭 JSON が無い / 音色キーが無い MML は
  `None` を返し、呼び出し側がそれを「既定プラグイン」として扱うこと
