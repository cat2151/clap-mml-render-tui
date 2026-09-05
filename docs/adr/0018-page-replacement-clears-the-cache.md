# ADR 0018: ページを全置換する経路は、その workspace のキャッシュ WAV を掃除する

- 状態: 採用（2026-09-04。Daily の 2 経路に実装済み。Persistent は未対応）
- 関連: [0011](0011-verification-and-baselines.md) /
  [0012](0012-live-clock-drift-is-absorbed-not-eliminated.md) /
  [0016](0016-daw-live-playback-slots-and-timeline.md) /
  [0019](0019-investigation-stage-acceptance.md)

## 何の話か

ユーザーが DAW を実際に動かして **「前日の音が混ざっている」** と報告した（2026-09-04）。
実ファイルとログで確定した事実:

- 今日の project（`daily_daw/current.json`）に中身のある行は **1 行だけ**（`row2`）。
  にもかかわらず**演奏ログは 7 行を鳴らしていた**（`sent=row2/i0,row3/i1,…,row8/i6`）
- キャッシュ WAV の長さが **3 世代のテンポ**を示していた。`(サイズ − 68) / 8` フレームから
  尾 96000 フレームを引いて逆算すると **BPM 120 / 113 / 107**。`daily_daw/archive/` の
  `t120` / `t113` / `t107` と完全に一致
- **つまり今日の timeline は BPM120 で刻みながら、行 3〜8 には BPM113 の素材を、
  その meas5 には BPM107 の素材を鳴らしていた**

## 原因（連鎖は 3 つで閉じている）

1. **演奏ループはファイルの存在しか見ない。**
   `daw/src/playback/live_cache.rs` の `ready_cache_wav_for_measure()` は
   `cache_wav_path(...)` に `path.is_file()` を掛けるだけ。演奏スレッドのクロージャが
   捕まえるのは `workspace_kind` **だけ**で、`cache` も `editor` も渡っていない
   （RT スレッドをロックから切り離す**意図的な設計**）。
   結果、**ファイルシステムが「何を鳴らすか」の唯一の真実**になっている
2. **ページを全置換する経路は WAV を消さない**（下記）
3. **空になった行は `CacheState::Empty` なので再レンダリングもされない**。
   `kick_all_pending()` は中身のあるセルしか投入しないので、古い WAV が今日のファイル名を占め続ける

キャッシュの「有効性」は **7 か所**に分散していて（ファイルの存在 / `CellCache::state` /
`rendered_mml_hash` / `rendered_measure_samples` / `generation` / `samples` /
永続化された `cached_measures[].mml_hash`）、**演奏経路が読むのは最も弱い 1 つ**。
しかも `types.rs` の `set_pending()` は「再 render 中でも演奏を止めないため、旧世代の
samples をここでは消さない＝ playback fallback 用の stale キャッシュとして扱う」と
**stale を残す方針を明示している**。この方針は「メモリ内・同一セル・再 render 中」のもので、
**「ディスク上・別の行・日をまたいで」へ一般化してはいけない。**

## 正しい不変条件

> **キャッシュ無効化は編集イベント駆動である。ページ全体を差し替える経路だけが、その規律を 1 度も通らない。**

「消す責任」は在る（`daw/src/cache.rs` の `invalidate_cell` / track0 変更 / chord 行変更 /
音色セル変更＝ `remove_file` が 4 か所）。素通りするのは次の 3 経路:

| 経路 | 場所 | 対応 |
|---|---|---|
| daily rollover | `daw/src/daily.rs` `rollover_daily_recovery()` | **塞いだ** |
| grid からの日中の全置換 import | `daw/src/grid_import.rs` `replace_with_grid_song()` | **塞いだ** |
| Persistent の読み込み | `daw/src/save.rs` `load_persistent()` | **残っている**（下記） |

> `remove_dir_all` で grep すると production は 0 件なので「誰も消さない＝消すのは新機能」に着地し、
> `remove_file` で grep すると「編集時だけ消す＝消すのは既存不変条件の回復」に着地する。
> **動詞の選択で正反対の設計判断が出る。** これが調査中に実際に起きた（[0019](0019-investigation-stage-acceptance.md)）。

## 決定

**掃除は `daw/src/cache.rs` の `clear_workspace_cache_wavs()` に 1 本化し、
呼び出しは全置換の 2 か所へ置く。共通の底には置かない。**

| 呼び出し元 | 関数 | ログ |
|---|---|---|
| `daw/src/daily.rs` rollover **成功の腕だけ**（`Ok(Created \| AlreadyExists)`） | `clear_daily_cache_after_rollover()` | `daily cache cleared: dir=…; removed=<n> wav` |
| `daw/src/grid_import.rs` `replace_with_grid_song()` | `clear_daily_cache_after_full_replacement()` | `grid import cache cleared: dir=…; removed=<n> wav` |

- **`*.wav` だけを消し、ディレクトリごとは消さない。** `Persistent` の置き場は `daily/` の
  **親**（`daw/src/cache.rs` の `Persistent => root` / `Daily => root/daily`）なので、
  取り違えたときに巻き込まないため
- **ログ行の綴りを 2 つに分けてある。** 同じ綴りにすると、**掃除を失った rollover を、
  日中の全置換の掃除が埋め合わせて見えてしまう**（実際に確かめた）
- 掃除の位置は `apply_project_snapshot_for_recovery()` の**あと**（＝ generation を進めたあと）・
  `kick_all_pending()` の**前**。`store_cache_job_samples()` は generation 不一致なら WAV を
  書かずに戻るので、**飛んでいる render job が掃除したそばから古い WAV を書き戻すことは無い**

### 共通の底（`apply_project_snapshot_state()`）へ置いてはいけない — 実測で確定

底に見えるが、`apply_project_snapshot_for_recovery()` の呼び出し元は 2 つしかなく、
片方（`daily.rs` の `apply_daily_recovery()`）は**直後に `restore_cache_from_metadata()` で、
いま消したはずの WAV を Ready として復元する**（Resume と rollover 失敗の経路）。

実際に底へ移して測ったところ **3 本が赤くなった**。特に
`daily::tests::workspace_entry::same_day_entry_restores_project_cursor_and_daily_cache_without_persistent_writes`
＝「**同じ日の再起動で前のキャッシュが Ready に戻らない**」。

同じ理由で、rollover の掃除は **`Ok` の腕だけ**に置く。`Err` の腕
（`keep_daily_after_rollover_failure()`）は前日のページを保持する経路で、前日の WAV を必要とする。
ここで消すと「archive も書けていないのにキャッシュも無い」状態を作る。

### 「昨日の続きを鳴らす」使い方は production に存在しない（掃除が失うものは実質ゼロ）

当初この直しは「『昨日の続き』を壊す」として保留されていた。**それは誤りだった**（[0019](0019-investigation-stage-acceptance.md)）。

1. `rollover_daily_recovery()` の成功の腕は archive を書き `daily_page_date` を更新するだけで、
   `apply_daily_recovery()` を**呼ばない**。前日の内容は editor に一切戻らない
2. 前日の内容が editor に戻るのは **Resume（同じ日の再起動）と rollover 失敗時**の 2 経路だけ
3. **Daily DAW から archive は開けない。** archive を開く `d` キーは Project overlay で、
   Project mode 自体が `workspace_kind == Persistent` に限定されている。
   しかも `open_project()` は全セルを `Empty` に落として焼き直すので、既存 WAV を 1 本も再利用しない
4. **Persistent 側は `daily/` を一切読まない**（置き場が分かれている）

## 塞いでいない穴（どれも設計判断。この直しとは直交）

| # | 内容 | 状態 |
|---|---|---|
| Persistent | `load_persistent()` に同型の穴。保存ファイルに無い行の WAV が残る | **番人テストが緑のまま残っている**（`daily::tests::stale_cache::a_persistent_load_also_keeps_a_cache_wav_for_a_row_missing_from_the_save_file`）。直したらこれが赤くなる |
| WAV の長さ検証 | `restore_cache_from_metadata()` は長さを見ず `measure_duration_samples()` を貼るだけ。**長さの違う WAV も Ready になる** | 未対応（テストが「壊れている振る舞いを記録した緑」で固定してある） |
| 書き込み中の WAV | `write_wav` はアトミックでない。書き込み中に読むと**無音**になる（エラーも出ない）。tmp + rename にすれば窓は消える（Windows 実測: DIRECT 127 回中 full 1 回 → RENAME 18 回中 full 18 回） | 未対応。**「前日の音が混ざる」の説明ではない別の不具合** |
| スロットの選び方 | `小節 index % SLOT_COUNT` なので**ループ長 5・9・13 は余裕 0**。定常ループは実害なしだが、**末尾の小節から演奏開始すると別の小節が鳴る**（margin −0.20 秒・実サーバーで再現） | 未対応。詳細と直し方は [0012](0012-live-clock-drift-is-absorbed-not-eliminated.md) |
| キャッシュ名に hash | `track{行}_meas{小節}.wav` に project 同一性も日付も hash も無い | 掃除で足りるならやらない。上位互換だがキャッシュが肥大する |

## 再発したら見るもの

```
python scripts/check_daw_cache_staleness.py            # 実キャッシュの世代混在（WAV の長さだけで決まる）
python scripts/check_daw_cache_staleness.py --log auto  # 実行ログの sent= と project の中身を突き合わせる
```

- 世代が混ざっていれば **exit 1**（長さの種類数と、project に無いのに WAV が在る行を挙げる）
- `--log auto` は **「rollover したのに `daily cache cleared` が無い」／「全置換 import したのに
  `grid import cache cleared` が無い」を NG にする**（`daw_log_sent_rows.py`）
- 掃除が空振りしていないかは**ログの `dir=`** を見る。名前空間は `init_cache_plugin_namespace()`
  （config 読み込み直後・`DawApp` を作るより前）で決まるので、実機では `Surge XT` になる

単体テストは `daw/src/daily/tests/stale_cache.rs` と `daw/src/grid_import/tests/stale_cache.rs`。
実機と同じ形（35 ファイル・長さ 2 種）を temp に作って 1 本残らず消えることと、
`*.wav` 以外（`notes.txt`）が残ることを見る。番人が 2 本ある:
`a_failed_rollover_keeps_yesterdays_page_and_its_cache_wav`（掃除を `Err` の腕や共通の底へ動かすと赤）と、
`a_rejected_import_into_a_persistent_daw_touches_no_cache_wav`（掃除をワークスペース判定より前へ出すと赤）。

## 罠

- **掃除は rollover / 全置換の瞬間にしか走らない。** 既に汚れている日は、その日いっぱい症状が残る。
  今すぐ消すならアプリを終了して `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\daily\*.wav` を手で消す
  （中身のある行は起動時に焼き直される）
- **ディスクを触る関数へ変えたら、その関数を呼ぶテストを全部 grep すること。**
  `replace_with_grid_song()` がディスクを触るようになった時点で、env を temp へ逃がしていない
  既存テスト 3 本が**実ユーザーのキャッシュを消す**ようになった。受け皿が
  `crate::input::tests::temp_local_dirs()`。**`daw_cache/` を通るテストは必ずこれを通す**
- **release バイナリは `CMRT_BASE_DIR` を見ない**（`#[cfg(test)]` 限定）ので、実機検証で
  `%LOCALAPPDATA%` を temp へ逃がすことはできない。**退避 → 実行 → md5 比較**の順で行う
- **`%LOCALAPPDATA%` の実キャッシュを「証拠」として当てにしない。** 古いキャッシュから始める
  再現がしたければ、実機のファイル名・長さを temp に自分で作ること
- **実機の rollover は `current.json` の `page_date` を過去日へ書き換えれば任意に起こせる。**
  実機の全置換 import を外から叩く入口は無い（実経路の `replace_with_grid_song()` はテストで通してある）
