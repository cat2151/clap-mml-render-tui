# ADR 0020: chord chart 画面は文字列を解釈しない

- 状態: 採用
- 関連: [0019](0019-investigation-stage-acceptance.md)

## 何の話か

`Ctrl+G` → `C` で開く chord chart 画面は、1 曲ぶんのコード進行の「構成」だけを
俯瞰・編集する。**構成の画面であって、シーケンサではない。** preview はカーソル行の
section 1 つ（または行内の chord 1 つ）を鳴らすだけで、曲全体は鳴らさない。

最初の実装はユーザー発言 1 文
「1 曲全体の構成を俯瞰できる、コード進行の構成のみに集中できる画面が欲しい」
から出発したのに、AI が keybind 20 種・小節倍率・進行の検証と `!` 表示・
既定曲のハードコードまで決め、それに**自分で「承認済み」ラベルを貼った**。
動作確認は不合格。ユーザーの言葉:

> そんな機能いらない。小さく始めるのに邪魔。ジャッジメントに邪魔、というより、
> いらない機能があるためジャッジメントはそもそもその時点で不合格

それを剥がした結果が現在の形。**この ADR が残すのは「何を持たないか」**である。

## 決定

### 1. 打った文字列をそのまま持つ。解釈しない

- コード進行（`degrees`）も曲頭の指定（`prefix` = `"Key=C BPM120"`）も、
  **検証せず、変換せず、そのまま保存して、そのまま描く。** 空文字も受ける。
- したがって「読めない進行」の概念が無い。`!` 印も赤字も出さない。
- 書式は chord2mml-rs の既存フォーマット。**この画面のためのパースを新規に書かない。**
  結果として `chord-chart` crate は `cmrt-chord` に依存しない（`cargo tree -p cmrt-chord-chart` で 0 件）。

`i` の進行編集 UI と試聴は host の共有 MML overlay が受け持つ。そこでは入力途中の
chord を演奏用に解釈するが、解釈結果や音色を `chord-chart` crate へ持ち込まない。
解釈できない入力も編集・保存できるという、この節のデータ契約は変わらない。

理由: 解釈を持った瞬間、この画面は「構成を俯瞰する」以外の責務を抱える。
書式の正しさは、実際に鳴らす側（演奏スコープ）が鳴らせるかどうかで判る。

### 2. 小節の概念を持たない

`measures_per_chord`（1 コードを何小節伸ばすか）と、そこから出る総小節数・
所要時間・開始時刻の列は**概念ごと削除した**。小節は chord2mml の記法（`|`）が持つ。
画面に別建ての倍率を置くと、同じことを 2 か所が持つ。

### 3. 曲は 1 つ。ハードコードした曲を持たない

- 保存する曲は常に 1 つ（`chord_chart.json`）。曲名も持たない。
- 保存ファイルが無い / 読めないときは、**画面を最初に開いた時点で**カタログから
  1 件だけ抽選して section 1 つと arrangement 1 行を作る（`g` 1 回ぶん）。
  抽選できなければ空のまま、理由を下段に出す。
- **進行をソースへ焼き込むことを禁ずる**（旧実装の `A = I-V-VIm-IV`）。
  画面が持つ進行は、カタログから引いたものか人間が打ったものだけ。
- 抽選を**起動時ではなく画面を開いた時**に置くのは、カタログが遅延取得で、
  キャッシュが無い初回は最大 20 秒待つため。chord chart を開かない人の起動を止めない。

### 4. キーは小さく保つ。増やすときは人間の承認が要る

キーの一覧は `README.ja.md` の「chord chart画面」にある。ここには持たない。
小文字の `p` は割り当てていない（試聴のトグルは `Shift+P` と `Space` の 2 つだけで、
chord カーソルの位置ではなく**行全体**のトグル）。`Shift+Tab` も割り当てない。

**キーの集合そのものをテストが等値比較で固定している**
（`chord-chart/src/ui/tests/help.rs` の
`the_help_teaches_exactly_the_keys_that_survived_the_reduction` と
`the_bottom_line_names_no_key_outside_that_set`）。廃止キーが 1 文字で戻っても落ちる。
**足すときはユーザーの承認を取ってから、この 2 つのテストを直す。**

### 5. preview を足しても 1. は越えない

`chord-chart` crate が返すのは opaque な文字列と stable な section id、preview 要求だけで、
編集 UI・文字列の解釈・音色・sender・永続化は host 側に置く。

- `prefix` から Key トークンを抜くのも、1 行を組み立てるのも **app 側の glue**
  （`app/src/tui/chord_chart_glue.rs`）。crate から `cmrt-chord` への依存は復活していない。
- Sections pane の `i` は `EditDegrees(SectionId)` を host へ返す。host は現在の degrees を
  Chord Chart 専用 syntax・Modal 1 行の共有 MML overlay で開く。`Enter` は trim した値を
  stable id の section へ確定・保存し、`Esc` は変更を破棄する。`n` と `b` は従来の
  `chord-chart` 固有 1 行入力欄のままにする。
- 画面が持つのは「いま何を鳴らすべきか」の要求（`PreviewRequest`）と
  「鳴っているか」の写しだけ。
- chord 単位の preview で**切るのは `cmrt-chord`**（`cmrt_chord::chord_source_ranges()`。
  `chord2mml_core::parse()` が返す `ParsedItem::Chord { source_range }` は元の文字列上の範囲）。
  画面が持つのは「いま行内の何番目か」（`chord_cursor`）と、glue が書き戻す範囲の写し
  （`set_chord_ranges`）だけで、degrees は一度も解釈しない。反転を描く `ui/degrees.rs` も、
  degrees を**桁として測る**以上のことをしない。
- 読めない degrees は範囲が 0 件になる。そのときは**その行を chord 1 個として扱う**
  （鳴らすのは行全体・反転しない）。`!` 印も赤字も出さない。
- overlay の入力中 preview、`Ctrl+Space`、patch 候補の preview は、いずれも Key 文脈を
  付けた chord progression 専用の厳密な変換を通す。読めない入力を MML として fallback
  再生せず、DAW chord cell 用の外側の `| ... |` も足さない。これは試聴の可否だけを決め、
  保存の可否は決めない。

### 6. patch と編集内容の正本は host が owner ごとに持つ

- 共有 MML overlay の owner は Global と Chord Chart を区別する。通常の `Ctrl+P` と
  Chord Chart は canonical patch を別々に持ち、`history.json` へ独立して保存する。
- Chord Chart の音色選択は進行編集 overlay 内の `Ctrl+T` から、既存と同じ patch catalog / selector
  を使う。候補移動は試聴だけ、`Enter` で確定、`Esc` で元の音色へ戻す。確定済みの音色は、
  その後 degrees 編集を `Esc` で破棄しても残る。
- 通常画面の行全体・chord 単体 preview も Chord Chart の canonical patch を使う。
  未選択の `None` は realtime play server の既定音色を意味する。
- `Shift+P` / `Space` の停止判定は enqueue 時刻から推測しない。sender が準備と送信を終えて
  公開した実演奏区間と、最後に送った command id が一致している間だけ「鳴っている」と扱う。

## この画面で学んだ検証の手（次に画面を作るときも同じでよい）

- **押した結果が buffer に出るか**で判定する。状態だけの assert は取りこぼす。
  1 行入力 overlay も「開く → Backspace → 1 文字ずつ → `Enter`」を実際に押せば全部見える。
- 「消したはずのものが残っていないか」は、**綴りを名指しで拒む assert では足りない**。
  1 文字のキー（`a` `e` `x` `d`）を素通しした。集合の等値比較にして初めて塞がる。
- 下段 1 行の要約は 80 桁端末で**桁数そのもの**を固定する（枠の内側 78 桁）。
  `?:help` の有無だけを見ていると、キーを 1 つ足した瞬間に末尾が黙って切れる。
- README の食い違いも機械で見つかる（図の 1 行を実装の定数と等値比較し、
  各行の表示幅を東アジア文字幅で数える）。
- 全角は buffer 上でセル 2 つ。照合は `ui/tests.rs` の `squeeze()` を通す。
- overlay の中身を見る assert は `help_overlay_bounds` の矩形だけを読む。
  画面全体だと裏のヘッダ（`Key=C BPM120`）を拾って必ず落ちる。
- overlay は**行数も**固定する。`centered_text_block_rect` は高さを `area.height` へ
  黙って `min` するので、ヘルプの行を 1 つ足すと 24 行端末で**末尾の行が消える**のに、
  枠は画面内に収まったままで幅のテストは通る（枠の内側の行数と `HELP_ROWS.len()` の等値比較で塞いである）。
