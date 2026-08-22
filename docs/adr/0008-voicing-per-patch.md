# ADR 0008: voicing は patch ごとに引く / 未知プラグインは poly とみなす

- 状態: 採用（2026-08-20 / 2026-08-22 に `VvpHeader` を追加）
- 関連: [0007](0007-patch-role-defaults-three-layers.md) /
  play-server `docs/adr/0005-dexed-mono-mode-is-poly.md` /
  play-server `docs/adr/0014-vvp-as-clap-state.md`

## 決定

`VoicingPolicy`（`app/src/tui/voicing.rs`）は Config 全体に 1 つではなく、
**`VoicingPolicies` として patch ごとに引き分ける。**

| 値 | 対象 | 振る舞い |
|---|---|---|
| `Sources` | Surge XT | shared JSON / ユーザー判定 / override の 3 層で引く。未判定は `None` |
| **`VvpHeader`** | **Vaporizer2** | **音色ファイル `.vvp` の `m_uPolyMode` を読む。読めなければ `None`** |
| `AssumePoly` | それ以外 | **全 patch を poly とみなす** |

`VoicingPolicies::for_patch()` は**方針とプラグインの組**を返す。display 文字列を実ファイルへ
戻すのに `CatalogPlugin::base` が要るので、方針だけでは足りない。

## `VvpHeader`: 答えが音色ファイルの中にある場合

Vaporizer2 は note dialect が MIDI だけで `NOTE_END` が返らないので、`Sources` の probe が
成立しない（play-server 側 ADR 0001）。**代わりに `.vvp` の先頭 4096 バイトに答えが書いてある**
（`m_uPolyMode`。実測で Mono 144 / Poly 316、未判定 0 件）。

- **`VoicingCache`（ユーザー判定 / `voicing_cache.json`）へは流し込まない。**
  そこは永続化される層で、`.vvp` の答えは音色ファイル側にある。流し込むと
  音色を差し替えたときに古い判定が残る。memo（`Arc<Mutex<HashMap<..>>>`）を独立させ、
  **永続化しない**
- **土台は遅延読み + memo で、先読みはその上の最適化。** memo があれば 1 音色 1 回しか開かない。
  先読み（`session.rs` の一覧読み込みスレッドが**一覧を公開するより前に**行う）は、
  初回フィルタの待ちを起動時のバックグラウンドへ前倒しするだけで、**正しさには要らない**。
  おかげで「先読みが間に合っていないときの倒れ方」を考えなくて済む
- 公開してから読む形にすると「その隙に wheel を回したときだけ Vaporizer2 が和音行から消える」
  という再現しにくい状態ができる。**公開より前**なのはそのため
- 実測: 460 件で warm 36〜39ms / cold 近似 51ms

## なぜ `AssumePoly` を「判定不能」にしなかったか

`matches_role` は `PatchRole::Chord` で `is_poly` を要求し、**未判定は外れ扱い**になる。
`None` を返すと**和音行の候補が必ず 0 件になり、画面ごと使えなくなる**。

poly と外した場合の実害（和音行で単音の音色が鳴る）のほうが軽いので poly 側へ倒した。
Dexed については実測（play-server 側 ADR）により「外した推測」ではない。

## Surge 専用リソースは他プラグインで取りに行かない

`voicing_shared_source` / `voicing_override_source` は `SourceSet::from_config()` が
`!cfg.is_surge_xt()` で `None` を返すので**ダウンロードもキャッシュ読みも発生しない**。

混在後は判定が「既定プラグインが Surge か」→「**カタログに Surge が載るか**」へ変わった。
`cmrt build-voicing-cache` は probe できない音色（cartridge）を対象から外す。

## 罠

- **`AssumePoly` は「判定していない」ではなく「poly と決めた」。**
  keyboard のステータス行も `detect: poly`（Cached）になる。
  Dexed では実測どおりだが、**未知のプラグインでは推測である**
- **`VvpHeader` は読めなければ `None`（未判定）にする。poly へ倒さない。**
  poly へ倒すと Mono の音色が和音行へ出て「最後の 1 音しか鳴らない」になる。
  chord 行の候補が 0 件になる `Sources` 側の事情（上の節）とは逆で、
  ここは**他のプラグインの候補が残るので 0 件にはならない**
- 判別材料は patch 文字列の形だけなので、**同じ形を扱うプラグインが 2 つ載ると区別できない**
- MIDI dialect には `note_id` が無いので `NoteEnd` が返らない。voicing probe は dialect が
  CLAP を含まない場合は**実行しない**（詳細は play-server 側 ADR）

## 別経路の裏取り（2026-08-22）

`m_uPolyMode` を読む方針が正しいかを、**音のほうから**確かめてある。
`cmrt render-mml --poly-check` が和音と単音 3 本をレンダリングし、
RMS の比（`energy_gain`）で mono / poly を判定する（[0011](0011-verification-and-baselines.md)）。

chord カテゴリの Mono 全 14 件 + Poly 12 件を通した結果、**poly と読み違えた mono は 0 件**。
ヘッダを読む経路と音を測る経路の 2 つが独立に同じ結論を出した。

なお**波形の一致では判定できない**（実測で外れた）。mono でも単音と波形が一致せず
（ノート優先やエンベロープ再トリガ）、poly のグラニュラ系は同じ MML でも毎回違う波形を出す。

## 壊れたら気づく場所（`VvpHeader` ぶん）

| テスト | 落ちたら |
|---|---|
| `app/src/tui/voicing/vvp_voicings/tests.rs::an_unreadable_preset_stays_undecided` | 読めない `.vvp` が poly へ倒れる（Mono が和音行へ出る） |
| 同 `::every_poly_mode_other_than_mono_is_poly` | 判定が綴りの一覧になっている（新しい Poly 値で壊れる） |
| 同 `::every_installed_preset_reports_a_voicing`（`#[ignore]`） | 実プリセット 460 件のどれかが読めない |
| `app/src/tui/voicing/tests.rs::a_three_plugin_catalog_reads_each_patch_form_its_own_way` | 3 方針の引き分けが壊れた |
