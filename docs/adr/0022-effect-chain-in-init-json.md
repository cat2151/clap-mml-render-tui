# ADR 0022: effect chain は init JSON の `"effects after instrument"` に持ち、TUI は解釈しない

- 状態: 採用
- 関連: [0003](0003-mml-patch-key.md) / [0009](0009-offline-entry-map.md) /
  [0013](0013-server-owned-audio-plugin-abstraction.md) /
  clap-mml-play-server `docs/adr/0020-audio-effects-are-baked-into-the-offline-render.md`

## 決定

DAW の track に挿す CLAP effect（TONE3000 / Surge XT Effects）は、その track の
**init セルの JSON** に次の形で書く。

```json
{"Surge XT patch": "patches_factory/Pads/Pad 1.fxp",
 "effects after instrument": [
   {"TONE3000 preset": "Bogner Fullstack"},
   {"Surge XT Effects preset": "Reverb 1/Cathedral 2.srgfx"}
 ]}
```

- 配列の順 = 信号の順（instrument → 先頭 → … → 末尾）
- 各要素は**キー 1 つのオブジェクト**。キーが plugin を、値が preset を決める
- キーの綴りも値の形も play server の catalog（`AudioEffectCatalog`）が持つ。TUI は catalog が返した
  `(json_key, value)` をそのまま書き、読むときも要素を値のまま出し入れする（`daw/src/mml/effect_chain.rs`）。
  **TUI に plugin 名で分岐するコードを置かない**（[0013](0013-server-owned-audio-plugin-abstraction.md)）
- 配列が空になったらキーごと消す。キーが無い = effect 無し
- 未知のキー・catalog に無い preset は render 前に**エラー**（判定は play server 側）。黙って dry で鳴らさない

## この形にした理由

- **cache key は init 込みの cell MML の hash**なので、chain を init JSON に置けば、変えたときの
  再 render が自動で走る。別の列や別ファイルに置くと、無効化の経路をもう 1 本持つことになる
- `"Surge XT patch"`（[0003](0003-mml-patch-key.md)）は**値の形**で plugin を判別しているが、
  effect ではそれができない。TONE3000 の値は preset 内の名前（ファイル名が uuid で読めない）で、
  拡張子が無い。だから**キー**で plugin を決める
- `{"plugin": "…", "preset": "…"}` の 2 キー形にすると、TUI が `plugin` の値を見て分岐する経路が
  生まれる。キー 1 つなら、TUI が持つのは「catalog の要素をそのまま書く」だけで済む

## offline render 側の置き方

- effect plugin の entry は instrument の entry 表と**別の表**
  （play-server `core-lib/src/effect_plugins.rs` の `EffectPlugins`。TUI はそれをそのまま使う）。
  instrument の「音色無指定なら先頭」の規則を effect に効かせないため
- catalog の走査も DLL のロードも、**最初に要るときまで遅らせて以後は保持**する。effect が無い環境で
  起動を待たせないため
- render は render-server 側が同じ `EffectPlugins` を持って行う（[0024](0024-offline-render-goes-through-the-render-server-only.md)）。
  TUI は `x` overlay の一覧表示のために `EffectPlugins::discover()` を呼ぶだけで、DLL はロードしない

## 残している論点

- keyboard / grid sequencer / MML overlay での effect。データの形はこの ADR のままでよい
- リバーブの尻尾は cell の長さで切れる（render を延長していない）

## 壊れたら気づく場所

| テスト | 落ちたら |
|---|---|
| `cmrt-daw` `mml::effect_chain::tests::reads_the_chain_in_order_without_interpreting_it` | TUI が要素を解釈し始めた |
| `cmrt-daw` `mml::effect_chain::tests::writing_an_empty_chain_removes_the_key` | 空配列が残り、`{}` の init セルが増える |
| `cmrt-daw` `mml::tests::build_cell_mml_keeps_the_effect_chain_array_from_the_init_cell` | render に渡る cell MML から chain が落ちた |
| `cmrt-daw` `input::tests::effect_chain::x_does_not_open_on_a_route_without_effects` | effect を持たない経路（テスト用）で書けてしまう |
| play-server `cmrt-core` `pipeline::tests::effects::unsupported_route_rejects_a_chain_before_rendering` | effect を持たない経路が chain を黙って dry で通した |
| `cmrt-render-core` `mml_with_resolved_embedded_patch_keeps_the_effect_chain` | 音色の解決で chain のキーが落ちた |
