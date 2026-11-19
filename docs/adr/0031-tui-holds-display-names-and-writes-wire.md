# ADR 0031: TUI の内部は表示名で持ち、MML と live IPC へ出す境界で wire にする

- 状態: 採用
- 決定者: user（patch 文字列 `<パス><拡張子>:<タグ>`、JSON キー `"patch"` / `"patch filter"`、旧キーはエラー、
  表示名は空タグの `:` を落とす、互換 reader を置かない）。変換を置く境界と live IPC の引数を wire にすることは AI の設計
- 置き換え: [0001](0001-patch-string-decides-the-plugin.md) / [0003](0003-mml-patch-key.md)
- 関連: play-server `docs/adr/0026-patch-tag-decides-the-owner.md`（文法と解釈の本体）

## 決定

1. 音色一覧・favorites・履歴のキー・画面の状態は**表示名**（空タグの `:` を落とし、非空タグは残す）で持つ
2. MML 先頭 JSON へ書くときは `cmrt_core::wire_from_display()`、読むときは
   `cmrt_core::patch_display_from_embedded_json()` を通す。どちらも play-server `core-lib` の再輸出で、
   TUI は patch 文字列の解釈を自前で持たない
3. 読み取りで旧キー `"Surge XT patch"`・区切りの無い旧形式は「音色なし」として扱う。既定音色に
   読み替えず、その MML を受け取った play server がエラーにする
4. `cmrt-realtime-play` の音色引数（`prepare_live_patch*`・standby・probe）は wire。表示名を持つ呼び出し側
   （mml-overlay の sink・grid sequencer・keyboard・catalog の計測と voicing 判定）が送る直前に変換する
5. `PatchPlugins::index_for_patch()` などカタログ内の routing は表示名で引く
6. SHM の `VERSION` は play-server と同時に 14

## 理由

- 表示名で持てば、既存の favorites・履歴のキーと音色一覧の照合がそのまま使える
- 変換を MML と IPC の出入口だけに置くと、画面ごとの状態や比較のコードを触らずに済む
- cache WAV の patch 文字列は play-server の `cache_wav_patch_with_slot()` が wire で返すので、
  live IPC の引数を表示名にすると、その経路だけ二重に `:` が付く。引数を wire に揃え、表示名を持つ側が変換する

## 却下した案

| 案 | 却下理由 |
|---|---|
| `cmrt-realtime-play` の中で表示名を wire へ変換する | cache WAV の wire と区別できず二重に `:` が付く。この crate に play-server core への依存も増える |
| 画面の状態ごと wire で持つ | favorites・履歴のキーと音色一覧の照合をすべて書き換えることになり、既存キーとも一致しなくなる |
| 旧キーを TUI 側で読み替えて表示する | 互換 reader が残り続ける。旧データは一度だけの移行で書き換える |

## 番人テスト

- `cmrt-render-core` `mml_with_resolved_embedded_patch_keeps_the_tag_and_leaves_legacy_patches_alone`
- `cmrt-daw` `the_init_cell_stores_the_wire_and_reads_back_the_display_name`
- `cmrt-daw` `current_track_patch_name_ignores_the_legacy_key_and_untagged_values`
- `cmrt-notepad` `record_patch_phrase_history_keys_by_display_name_and_skips_legacy_lines`
