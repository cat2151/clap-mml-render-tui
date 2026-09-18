# ADR 0021: auto voicing の Bass は chord layer の最低音と unison になってよい

- 状態: 採用
- 関連: [0020](0020-chord-chart-does-not-interpret-strings.md)

## 何の話か

`cmrt_chord::auto_voice_with_key` は、和音の転回（chord layer）と Bass の octave を
別々に選ぶ。Bass の候補は canonical（構造上の root の 1 octave 下）とその ±12 で、
「chord layer の最低音より下」を hard constraint にしていた。

## 決定

hard constraint を **「chord layer の最低音以下」** にする。Bass と chord layer の
最低音が同じ note number（別音色での root の重複）になることを許す。

## 理由

`<` のままだと、canonical が chord layer の最低音と同音になった 1 chord で候補が
octave down 1 つだけになる。Score は 7 半音超の跳躍（`jump_excess`）を octave move 数より
優先するので、その 1 音へ跳ばずに着くために **前後の chord まで octave down に引きずられる**。
Arrangement で section を連結して voice すると、同じ IVM7 が section によって
F3 / F2 に分かれる形で現れた（VIM9 の chord layer が A3 始まりになり、canonical A3 が消えた）。
unison を許せば、両 section とも section 単独で聴いたときと同じ Bass になる。

## 却下した案

- **Score の優先順位を octave move 優先へ入れ替える。** 引きずりは止まるが、canonical を
  失った chord 自身は octave down のままで、直前からの 10 半音跳躍が残る。
- **chord layer に最低音の下限 penalty を足す。** chord layer の DP は octave 違いの 2 解が
  ほぼ同点で、ごく小さい penalty でも進行全体が 1 octave 反転する。lever にならない。
- **Arrangement を section ごとに独立して voice する。** 曲順を 1 本で voice する仕様
  （section 境界の voice leading）を捨てることになる。
- **Bass lane を 1 octave 下げる（tonic anchor C3 → C2）。** 衝突しなくなるだけで、
  DAW / grid sequencer を含む全画面の Bass 音域が変わる。別の論点として残す。

## 結果

- `chord/src/auto_voicing/bass.rs` の候補条件が `<=` になる。
- 全 12 key のカタログ性質（span ≤ 16、同 pitch class の spread ≤ 12、tonic 固定）は変わらない。
- Bass の候補が空になって選択不能になる条件が 1 半音ぶん狭まる。
