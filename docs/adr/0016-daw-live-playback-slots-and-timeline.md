# ADR 0016: DAW の live 演奏は cache-player のスロットと timeline で鳴らす

- 状態: 採用（2026-09-03）
- 関連: [0010](0010-two-repo-layout.md) / [0011](0011-verification-and-baselines.md) /
  [0012](0012-live-clock-drift-is-absorbed-not-eliminated.md) /
  clap-mml-play-server `docs/adr/0018-patch-load-must-not-spin-the-plugin.md` /
  clap-mml-play-server `docs/adr/0019-cache-player-slot-headroom.md`

## 何の話か

2026-09-03 に DAW の演奏を play server の live mix へ移した（TUI `82f26a8` / play server `10393ec`）。
gain の即時反映という目的は達成したが、**演奏が「モタる」「小節ごとにぶつ切りに聴こえる」**
という別の問題が出た。実測ログで原因は 2 つに割れた。

| 症状 | 実測 | 原因 |
|---|---|---|
| モタり | note on の小節間隔が 100352〜103424 サンプル（理想 102400・**振れ幅 64ms**・512 サンプル量子化） | DAW だけが timeline を通っていなかった（`cmrt-live: ... timeline=false`）。`FastMidiEvent { offset_frames: 0 }` で「届いたブロックで鳴らせ」と送っていた |
| ぶつ切り | 小節境界に 100ms 強の完全な無音（`prepare_ms` が毎小節 98.6〜129.8ms、異常値 6117ms） | 先読みが無く、境界に着いてから 7 track ぶんの `prepare_live_patch` を**順次同期**で発行していた。さらに cache-player の `refresh_buffer()` が buffer 差し替えのたび `voices.stop_all()` を呼び、track ごとにバラバラに前小節の音が切れていた |

**大前提（変更不可）:** 「CLAP と、server 内蔵 audio プラグイン（cache-player）で鳴らす」
という仕様は維持する。自プロセスで混ぜる rodio 経路へ戻すことは検討しない。

## 決定

### 1. 1 演奏 track = 1 live instance は維持する。複数 WAV を持つのは時間方向だけ

「1 instance に全 track の WAV を載せて voice でアサインする」案は**採らない**。
gain が `set_live_instance_gain_db(instance_id, gain_db)` ＝ **instance 単位**なので、
まとめると今回の変更の主目的だった「mixer の per-track gain が混ぜる直前に効く」が壊れる。

複数 WAV を抱える必要があるのは **小節方向だけ**。これがスロット化。

### 2. voice は `Arc<CacheBuffer>` を自分で握る（スロット index を参照しない）

スロットを差し替えても、鳴っている voice は自分が握った `Arc` を鳴らし続ける。
これで `stop_all()` が要らなくなり、余韻が切れなくなる。

**罠: RT スレッドで `Arc` を drop しないこと。** 最後の参照が RT で消えると
`Vec<Vec<f32>>` の解放（数 MB の free）が RT スレッドで走る。使い終わった `Arc` は
返却キューへ push し、main thread が回収する（play server 側 `cache-player/src/graveyard.rs`）。
**`process` 中に一切の確保・解放を起こさない**という `VoiceBank` の既存方針を崩さない。

### 3. スロット番号は patch 文字列へ入れる。SHM のプロトコル版は上げない

patch 文字列は既に「patch 文字列 → プラグイン」の 1 本道
（[0001](0001-patch-string-decides-the-plugin.md) / play server `docs/adr/0007`）を通っているので、
SHM のレイアウトも `PreparePatch` の形も変えなくてよい。**両 repo 同時のプロトコル変更が要らない**
のが大きい（SHM v6 / v9 / v10 で「片方だけ古い」事故を繰り返している）。

綴りは**プレフィクス形** `slot=<n>;<パス>`。`Path::new(patch).extension()` が `"wav"` の
まま取れる必要があるので、`...\track2_meas3.wav#1` のようなサフィックス形は
**拡張子判定を壊すので不可**。綴りの単一ソースは play server の
`core-lib/src/cache_wav.rs` の doc コメント。

### 4. note on は timeline へ載せ、先読みは同期 `prepare_live_patch` のままでよい

`prepare_standby_patch`（非同期・protocol v10）は bank 切替とセットの仕組みで cache-player には過剰。
timeline 化が済めば、演奏スレッドが `prepare_live_patch` で 100ms ブロックしても
**note on は既に予約済み**なので音には出ない。

### 5. スロット数

当初 2（現在の小節＋次の小節。1 本 1.6MB × instance 数で 16 instance ≒ 51MB）。
その後 [0012](0012-live-clock-drift-is-absorbed-not-eliminated.md) で **4** へ増やした
（クロック先行の吸収）。**余韻の長さとスロット数は無関係**で、スロットは
「同時にロードしておける本数」であって鳴っている音を保持する仕組みではない（決定 2）。

## 結果（release サーバー・7 track・小節 2400ms）

| | 対策前 | 対策後 |
|---|---|---|
| 小節境界の state load (`prepare_ms`) | 毎小節 98.6〜129.8ms（異常値 6117ms） | **2 小節目以降 0.0ms** |
| 小節境界の note on (`note_on_ms`) | 0.6〜1.1ms | **2 小節目以降 0.0ms**（境界で 1 バイトも送らない） |
| 発音位置のジッター | −42.7〜+21.3ms（振れ幅 64ms） | **0 サンプル**（`at_frames` の差が 115200 ちょうど・`late_total=0`） |
| 先読みの所要時間 (`next_ms`) | —（境界に居た） | 117〜158ms（小節の **6.3%**。debug サーバーだと 22.0%） |
| 演奏中の音の途切れ | 小節境界に 100ms 強の無音 | **0 frames**（`dropouts=[]`） |

再現は 1 コマンド（起動・WAV 生成・テスト・**サーバー停止**まで閉じてある）:

```
python scripts/verify_daw_playback_timing.py --server-profile release
```

出力サンプル列そのものは play server 側の
`cache-player/src/tests/measure_boundary.rs` が押さえている（小節 1 にランプ WAV、
小節 2/3 に定数 WAV を載せ、3 小節ぶんの**全フレーム・両チャンネル**を厳密比較。
`stop_all()` が戻れば赤くなる）。

## 直っていないもの（**仕様として残す**）

- **先読みが外れた小節で、1 つ前に予約した小節の頭が一瞬鳴る。** 演奏中に AB リピートや
  小節数を変えたときだけ。予約は既にサーバーへ届いていて取り消せない。ログの `preload=miss` が目印
- **演奏開始の 1 小節目だけ、キャッシュのロード＋先行時間ぶん遅れて鳴り始める。** 2 小節目以降は遅れない
- **キャッシュ WAV の余韻（小節長の約 2 倍ある）は、次の小節の頭で消える。** スロットの
  差し替えでは切れないが、同じ instance に次の小節の note on が来れば新しい voice が始まる。
  リバーブの尻尾が小節をまたいで残る形にはなっていない
- **debug ビルドのサーバーでは先読みの最中に音が途切れる**（7 track・4 小節で 29376 frames = 0.61 秒）。
  WAV デコードが release の 6〜7 倍重い。→ [0017](0017-play-server-binary-resolution.md)

## 残っている構造上の弱さ（今回の原因ではない）

DAW の先読みは**同期 `prepare_live_patch`**（`daw/src/playback/live_cache/send.rs`）で、
**演奏中の bank へ直接**載せている。play server 側 `player/worker/live_mix.rs` の
`live_render_requests` は「先読み中の bank に属する instance は render 要求を出さない。
**正規の経路では起きない（先読みは非演奏 bank へしか来ない）**」と書いており、
**DAW の経路だけがこの前提を外れている。**

grid sequencer は 2 bank の standby（`begin_standby` / `poll_standby`）を使っており、
先読みは非演奏 bank へ行く。つまり **「先読みが小節長の 20% 超を占めると音に出る」という
耐性の低さ**が残っている。直す案は「DAW も standby の 2 bank 経路へ移す」で、
これは新設計ではなく**既にある仕組みを DAW が使っていないだけ**。着手するなら別資料を立てること。

## 罠（この経路を触るときに踏むもの）

- **`apply-midi ... clock=` の行は timeline 経路には出ない。** ジッターの判定は
  サーバーログではなく、小節ログの `at_frames` の差と `timing_metrics().late_events_total == 0` で行う
- **サーバーのサンプルクロックは、最初の timeline イベントまで動かない**
  （[0012](0012-live-clock-drift-is-absorbed-not-eliminated.md) の原点合わせがこれに乗っている）
- **`set_patch` は同じ綴りなら何もしない。** スロットを変えずに WAV だけ変えたつもりでも効かない
- **「予約が揃っている」ことと「そのとおり鳴った」ことは別。** 帳簿が全部 0 でも違う小節が鳴りうる
  （[0012](0012-live-clock-drift-is-absorbed-not-eliminated.md) の「再発したら最初に見るもの」）
