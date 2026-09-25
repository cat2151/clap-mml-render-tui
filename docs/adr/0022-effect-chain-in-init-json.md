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
   {"Surge XT Effects preset": "Reverb 1/Cathedral 2.srgfx", "bypass": true}
 ]}
```

- 配列の順 = 信号の順（instrument → 先頭 → … → 末尾）
- 各要素は**plugin を決めるキー 1 つ＋任意の `bypass`** を持つオブジェクト。キーが plugin を、値が
  preset を決める。`bypass` が `true` の段は render に渡す前に chain から落ちる（`apply` 自体は
  無変更のまま）。TUI が読み書きするのは `bypass` キーだけで、他のキーの意味は解釈しない
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

## EFFECT CHAIN の試聴の経路（cache → LIVE）

`x` overlay の試聴（`daw/src/input/effect_chain/preview.rs`）は、対象 track だけの MML を作り、

1. overlay preview cache に一致する音があれば、backend に関係なくそれを鳴らす
2. 無ければ MML overlay sender（`cmrt_mml_overlay::MmlOverlaySender`）で LIVE へ送る。live instance が
   音色と chain を同じ準備で受け取り、effect 付きで鳴らす（play-server ADR 0020「live instance の chain」）

周辺候補の先読み render は続けるので、操作しているうちに cache が埋まり、cache から鳴る割合が増える。

- 試聴の MML → 音色 + chain + 演奏の変換は `cmrt_mml_overlay::live_line` の 1 か所。CLI
  `cmrt live-line-check` も同じ関数を通す（CLI と画面で入力が食い違うと、CLI の計測が画面の音を表さない）
- cache miss で offline render へ切り替えない。LIVE の準備が失敗したら log と overlay に 1 行出して鳴らさない。
  play server が無い構成でも同じ欄に出す。切り替えると、失敗が「遅れて鳴る」に化けて見えない
- 移動を受けた時点で、LIVE で鳴っている前の候補（音色の release と effect の余韻）を 50 ms で fadeout してから
  次の候補を鳴らす（`MmlOverlaySender::fade_out_line`。長さは固定値）。fadeout は sender の command 列に並べず
  呼び出したスレッドから送る。列の前に次の候補の準備（chain 付きで数百 ms）があっても待たないため。
  このため前の候補が消えてから次の候補が鳴るまでは無音になる
- `stop_all`（server の全NoteOff）は挟まない。出力リングを捨てるので、fadeout も release も段差で切れる。
  通常の preview と演奏の開始では LIVE の試聴を止める
- 移動先が cache の候補でも LIVE の前の候補は fadeout する。cache の音（rodio）は次の試聴で即断のまま（高速さを優先）。
  fadeout は EFFECT CHAIN の試聴だけで、grid sequencer・Chord Chart・MML overlay の行の切り替えは release のまま消える
- 音色の違う候補はもう一方の bank の instance へ先読みの経路で準備する（鳴っている bank を止めないため）。
  準備は同じ instance の effect の余韻を切るが、2 つ前の候補はその時点で fadeout 済みなので段差にならない
- LIVE の試聴には track の音量（mixer の dB）を掛けていない。cache から鳴る音（焼き込み）とは音量が違いうる

## 残している論点

- keyboard / grid sequencer / MML overlay（notepad の行）での effect。live instance は chain を持てるので、
  送る側が `LivePatch` に chain を入れれば足りる
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
| `cmrt-daw` `input::tests::effect_chain::preview::add_overlay_previews_on_open_and_when_the_cursor_changes_the_candidate` | cache hit でも LIVE へ送る、または miss で送らない |
| `cmrt-daw` `input::tests::effect_chain::preview::a_failed_live_preparation_is_shown_until_the_next_preview` | LIVE の準備の失敗が表示されない、または前の失敗が残る |
| `cmrt-daw` `input::tests::effect_chain::preview::a_normal_preview_and_play_stop_the_live_preview` | 通常の preview・演奏で LIVE の試聴が残る、または fadeout する |
| `cmrt-daw` `input::tests::effect_chain::preview::moving_between_live_candidates_fades_out_the_previous_one_before_preparing` / `moving_from_a_live_candidate_to_a_cached_one_fades_out_the_live_one` | 移動で前の候補が fadeout されない、または準備の後ろに並ぶ |
| `cmrt-mml-overlay` `sender::tests::fade_out::the_fadeout_does_not_wait_for_the_next_line_to_load` / `playing_lines_alone_never_fades` | fadeout が準備を待つ、または行の切り替えだけで fadeout する |
| `cmrt-mml-overlay` `live_line::tests::the_patch_and_the_chain_come_from_the_leading_json` | 試聴の MML から音色か chain が落ちた |
