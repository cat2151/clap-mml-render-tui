# ADR 0028: patch selector の一覧は、どの Role でも Preset 欄の順に並べる

- 状態: 採用
- 決定者: user（Role=ALL でも各 Role でも、一覧を Preset の順に並べること。`sort_for_selector` を変えて並べること）
- 関連: [0007](0007-patch-role-defaults-three-layers.md)

## 決定

- `sort_for_selector` の最上位のキーを `preset_rank` にする。同じ順位の中は、Category の無い音色を末尾へ送り、残りを Category / plugin / patch 名の順に並べる。
- `preset_rank` は、Role=ALL の Preset 欄に並ぶ組み込み preset の位置を返す（Role の順に並び、Role の中は組み込み preset の順）。
  組み込み preset に入らない音色はその Role の preset の後ろに、Role の無い音色は最後に置く。
- 全体を1つの順で並べるので、Role=ALL でも各 Role の ALL でも、一覧は Preset 欄と同じ順になる。
- 順位を決めるのは組み込み preset だけ。user preset・`★ Favorite`・`Drum kit` は順位に使わない。
- 順位は host が持つ `PatchRoleIndex` から引く。cat-plugin-player は plugin ごとに作った index から引く。

## 理由

- user が「常に全てが preset 順に並んでいるのが自然」と判断した。Category 順のままだと、同じ preset の音色が一覧のあちこちに散らばる。
- 組み込み preset は1音色に1つだけ決まる（`preset_label_of`）ので、順位が1つに定まる。user preset は正規表現なので重なることがあり、順位が決まらない。

## 却下した案

- **cat-plugin-player 側だけで並べ替える。** cmrt の selector と並び順が食い違い、同じ catalog が画面によって違う順で出る。
- **各 Role の ALL だけ Preset 順にし、Role=ALL は Category 順のまま残す。** user が、どの Role でも Preset 順にすると決めた。

## 結果

- 一覧は Role の順（Bass → Chord → Lead → Drum → Triggered → Etc）に並ぶ。Chord の中では strings が pad より先に来るなど、組み込み preset の順に並ぶ。
- 組み込み preset の並びを変えると、一覧の並びも変わる。
- 35738 件の catalog で、並べ替えを含む cat-plugin-player の Snapshot 構築は 0.86 秒（別スレッドで動く）。
