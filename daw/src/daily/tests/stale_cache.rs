//! **日付が変わったときに前日のキャッシュ WAV がどうなるか**を固定する
//! （`docs/adr/0018-page-replacement-clears-the-cache.md`）。
//!
//! 陳腐化は 2 段で成立していた。
//!
//! 1. **消さない** … `rollover_daily_recovery()` はアーカイブを書くだけで、
//!    `daw_cache/<plugin>/daily/` に触らなかった
//! 2. **上書きもされない** … rollover 後のページは空なので `sync_cache_states()` が
//!    全セルを [`CacheState::Empty`] にし、`kick_all_pending()` が 1 件も投入しない
//!
//! この 2 つが揃うと、前日のファイルが今日のセルの名前
//! （`track{行}_meas{小節}.wav`。**日付も hash も入らない**）を占め続ける。
//! 「見つかったら鳴る」ところは Stage 1 の
//! `playback::live_cache::tests::cache_lookup` が固定済みなので、ここでは扱わない。
//!
//! **Stage 7 の (a) が 1 段目を断った。** いまここに並ぶのは
//!
//! - rollover 成功時は前日の WAV が**消えている**（(a) の本体）
//! - rollover **失敗**時は前日の WAV が**残っている**（前日のページを復元する経路なので必須。
//!   掃除を `Err` の腕へ動かされないための番人）
//! - 2 段目（空行は再レンダリングされない）は**まだそのまま**。ただし 1 段目が消えたので
//!   「上書きされるのを待っている古い WAV」自体が存在しない
//! - **Persistent 側の穴はまだ空いている**（候補 (b')）。その 1 本だけが直し漏れの番人

use std::path::{Path, PathBuf};
use std::sync::mpsc::TryRecvError;

use super::*;
use crate::tracks::FIRST_PLAYABLE_TRACK;
use crate::CacheState;

const YESTERDAY: &str = "2026-09-03";
const TODAY: &str = "2026-09-04";

/// テスト用の app dir。`set_local_dir_envs` が作る app ディレクトリと同じ場所にする
/// （キャッシュ WAV の置き場は env 経由、recovery JSON は `config_app_dir` 経由で決まるため）。
fn config_app_dir(temp: &TempDirectory) -> PathBuf {
    temp.path().join("clap-mml-render-tui")
}

/// 前日のページを作る: 行 `row` に中身があり、その小節の WAV も焼けている状態。
///
/// 戻り値は焼けた WAV のパス。
fn write_yesterdays_page_with_a_rendered_row(config_app_dir: &Path, row: usize) -> PathBuf {
    let mut app = build_daily_app(config_app_dir);
    app.daily_page_date = Some(YESTERDAY.to_owned());
    app.editor.data[row][1] = "cdef".to_owned();
    app.sync_cache_states();

    let wav = crate::cache::cache_wav_path(WorkspaceKind::Daily, row, 1)
        .expect("a playable row at measure 1 has a cache path");
    cmrt_core::write_wav(&[0.25, -0.25], 44_100, &wav).unwrap();

    let mml_hash = cmrt_history::daw_cache_mml_hash(&app.build_cell_mml(row, 1));
    {
        let mut cache = app.cache.lock().unwrap();
        cache[row][1].state = CacheState::Ready;
        cache[row][1].rendered_mml_hash = Some(mml_hash);
    }
    app.save_daily_recovery().unwrap();
    wav
}

/// rollover を通ったことをログで確かめる。
///
/// `initialize_daily_workspace()` は失敗しても黙って「fresh start」へ落ちるので、
/// **これを見ずに「WAV が消えている」だけ assert すると、経路違いでたまたま緑になる。**
fn assert_rollover_actually_happened(app: &DawApp, config_app_dir: &Path) {
    let log = app.log_lines.lock().unwrap();
    let expected = format!("daily rollover: {YESTERDAY} -> {TODAY}; archive=");
    assert!(
        log.iter().any(|line| line.starts_with(&expected)),
        "the test must go through the real rollover path; log={log:?}"
    );
    assert!(
        log.iter()
            .all(|line| !line.contains("daily recovery failed")),
        "a silent fresh start would make every assertion below meaningless; log={log:?}"
    );
    drop(log);
    assert!(
        daily_archive_path(config_app_dir, YESTERDAY)
            .unwrap()
            .exists(),
        "the archive must be written before the cache is thrown away"
    );
}

/// 掃除のログ行を取り出す（`--log auto` の判定が見るのと同じ行）。
fn cache_cleared_log_line(app: &DawApp) -> String {
    let log = app.log_lines.lock().unwrap();
    log.iter()
        .find(|line| line.contains("daily cache cleared: "))
        .unwrap_or_else(|| panic!("the cleanup must leave a machine-checkable line; log={log:?}"))
        .clone()
}

/// **Stage 7 の (a) の本体。**
///
/// 日付が変わったら、前日のキャッシュ WAV は消える。
/// ファイル名に日付も hash も入らないので、消さない限り前日のファイルが
/// 今日のセルの名前を占め続け、演奏ループ（ファイルの存在しか見ない）がそれを鳴らす。
///
/// 掃除を外す・`Err` の腕へ動かすと `stale_wav.is_file()` が赤くなる。
#[test]
fn a_daily_rollover_deletes_yesterdays_cache_wav_from_disk() {
    let temp = TempDirectory::new("rollover-deletes-wav");
    let _env_guard = cmrt_history::test_support::set_local_dir_envs(temp.path());
    let config_app_dir = config_app_dir(&temp);
    let row = FIRST_PLAYABLE_TRACK + 1;
    let stale_wav = write_yesterdays_page_with_a_rendered_row(&config_app_dir, row);
    assert!(stale_wav.is_file(), "the fixture must actually exist first");

    let mut app = build_daily_app(&config_app_dir);
    app.initialize_daily_workspace(TODAY);

    assert_rollover_actually_happened(&app, &config_app_dir);
    assert!(
        !stale_wav.is_file(),
        "yesterday's wav must not survive a successful rollover"
    );
    let cleared = cache_cleared_log_line(&app);
    assert!(
        cleared.contains("removed=1 wav"),
        "the log must say how many files went away; line={cleared}"
    );
}

/// **掃除が `daily/` の中を全部さらう**ことを、実機と同じ形のファイル名で固定する。
///
/// ユーザーの実キャッシュは `track2_meas1..5` が今日の世代、`track3..8_meas1..5` が
/// 前日・前々日の世代という 35 ファイルだった（ADR 0018 の実測）。
/// **世代が混ざっていても 1 本残らず消える**ことをここで見る
/// （残ると `check_daw_cache_staleness.py` が「長さが 2 種類以上」で赤くなる）。
#[test]
fn a_daily_rollover_leaves_no_wav_behind_even_when_generations_are_mixed() {
    let temp = TempDirectory::new("rollover-clears-all");
    let _env_guard = cmrt_history::test_support::set_local_dir_envs(temp.path());
    let config_app_dir = config_app_dir(&temp);
    let row = FIRST_PLAYABLE_TRACK + 1;
    write_yesterdays_page_with_a_rendered_row(&config_app_dir, row);

    let daily_dir = crate::cache::ensure_workspace_cache_dir(WorkspaceKind::Daily).unwrap();
    // 世代の違いは長さで出る（今日 = 短い / 前日 = 長い）。
    for track in FIRST_PLAYABLE_TRACK..FIRST_PLAYABLE_TRACK + 7 {
        for measure in 1..=5 {
            let samples = vec![0.1_f32; if track % 2 == 0 { 64 } else { 96 }];
            cmrt_core::write_wav(
                &samples,
                44_100,
                daily_dir.join(format!("track{track}_meas{measure}.wav")),
            )
            .unwrap();
        }
    }
    let keep_me = daily_dir.join("notes.txt");
    std::fs::write(&keep_me, b"not a wav").unwrap();

    let mut app = build_daily_app(&config_app_dir);
    app.initialize_daily_workspace(TODAY);

    assert_rollover_actually_happened(&app, &config_app_dir);
    let left: Vec<String> = std::fs::read_dir(&daily_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".wav"))
        .collect();
    assert!(
        left.is_empty(),
        "a mixed-generation daily cache must be emptied; left={left:?}"
    );
    assert!(
        keep_me.is_file(),
        "only *.wav is swept, so anything else in the directory survives"
    );
}

/// **番人。** rollover が**失敗**したときは前日の WAV を消してはいけない。
///
/// `keep_daily_after_rollover_failure()` は前日のページを editor へ復元し、
/// `restore_cache_from_metadata()` がその WAV を [`CacheState::Ready`] として引き直す。
/// ここで消すと「archive も書けていないのにキャッシュも無い」状態になり、
/// 前日の作業が音として完全に失われる。
///
/// 掃除を `Ok` の腕から共通の底や `Err` の腕へ動かすと、このテストが赤くなる。
#[test]
fn a_failed_rollover_keeps_yesterdays_page_and_its_cache_wav() {
    let temp = TempDirectory::new("rollover-failure-keeps-wav");
    let _env_guard = cmrt_history::test_support::set_local_dir_envs(temp.path());
    let config_app_dir = config_app_dir(&temp);
    let row = FIRST_PLAYABLE_TRACK + 1;
    let wav = write_yesterdays_page_with_a_rendered_row(&config_app_dir, row);
    // archive ディレクトリの場所に file を置いて create_dir_all を失敗させる
    // （`workspace_entry::rollover_failure_keeps_old_page_and_invalid_recovery_starts_fresh`
    //  と同じ手口）。
    std::fs::create_dir_all(daily_feature_root(&config_app_dir)).unwrap();
    std::fs::write(
        daily_archive_root(&config_app_dir),
        b"blocks directory creation",
    )
    .unwrap();

    let mut app = build_daily_app(&config_app_dir);
    app.initialize_daily_workspace(TODAY);

    let log = app.log_lines.lock().unwrap().clone();
    assert!(
        log.iter()
            .any(|line| line.starts_with("daily rollover failed:")
                && line.contains(&format!("keeping {YESTERDAY}"))),
        "the test must go through the failure path; log={log:?}"
    );
    assert!(
        log.iter().all(|line| !line.contains("daily cache cleared")),
        "the cleanup must never run when the archive could not be written; log={log:?}"
    );
    assert_eq!(app.daily_page_date(), Some(YESTERDAY));
    assert_eq!(app.editor.data[row][1], "cdef");
    assert!(
        wav.is_file(),
        "the restored page still needs yesterday's wav"
    );
    assert!(
        app.cache.lock().unwrap()[row][1].state == CacheState::Ready,
        "restore_cache_from_metadata() puts it back as Ready, so the wav must be there"
    );
}

/// **2 段目はまだそのまま**（ただし 1 段目が消えたので実害は無い）。
///
/// rollover 後のページは空なので、その行は今日 1 度も再レンダリングされない。
/// 空セルは [`CacheState::Empty`] で `kick_all_pending()` に拾われないからで、
/// これは Stage 7 の (a) では変わっていない。
/// **変わったのは「上書きされるのを待っている古い WAV」がもう存在しないこと。**
#[test]
fn a_daily_rollover_leaves_empty_rows_unrendered_but_the_stale_wav_is_already_gone() {
    let temp = TempDirectory::new("rollover-never-replaces-wav");
    let _env_guard = cmrt_history::test_support::set_local_dir_envs(temp.path());
    let config_app_dir = config_app_dir(&temp);
    let row = FIRST_PLAYABLE_TRACK + 1;
    let stale_wav = write_yesterdays_page_with_a_rendered_row(&config_app_dir, row);

    let (mut app, cache_rx) = build_daily_app_with_cache_jobs(&config_app_dir);
    app.initialize_daily_workspace(TODAY);

    assert_rollover_actually_happened(&app, &config_app_dir);
    assert!(
        app.editor.data[row][1].is_empty(),
        "rollover does not call apply_daily_recovery(), so today's page starts blank"
    );
    assert!(
        app.cache.lock().unwrap()[row][1].state == CacheState::Empty,
        "sync_cache_states() marks a blank cell Empty, not Pending"
    );

    app.kick_all_pending();

    assert!(
        matches!(cache_rx.try_recv(), Err(TryRecvError::Empty)),
        "nothing is queued, so nothing would ever overwrite yesterday's wav"
    );
    assert!(
        !stale_wav.is_file(),
        "so the wav must already be gone; otherwise it would sound forever"
    );

    // 「受け口が死んでいるから空」ではないことを 1 回示す。
    // 中身を入れれば同じ経路で同じセルの job がちゃんと流れてくる。
    app.editor.data[row][1] = "cdef".to_owned();
    app.sync_cache_states();
    app.kick_all_pending();
    let job = cache_rx
        .try_recv()
        .expect("a cell with content is queued, so the empty result above is not vacuous");
    assert_eq!((job.track, job.measure), (row, 1));
}

/// **Persistent ワークスペースにはまだ同じ穴がある**ことを記録した緑。
///
/// `load_persistent()` は全セルを clear してから保存ファイルを適用するが、
/// `daw_cache/` には触らない。保存ファイルに載っていない行の WAV は残り、
/// その行は `Empty` なので再レンダリングもされない。
///
/// **Stage 7 の (a) は Daily の rollover 経路だけを直したので、ここは緑のまま残る。**
/// これは直し漏れの番人であって、候補 (b') を実装したときに赤くなるのが正しい。
#[test]
fn a_persistent_load_also_keeps_a_cache_wav_for_a_row_missing_from_the_save_file() {
    let temp = TempDirectory::new("persistent-keeps-wav");
    let _env_guard = cmrt_history::test_support::set_local_dir_envs(temp.path());
    let config_app_dir = config_app_dir(&temp);
    let kept_row = FIRST_PLAYABLE_TRACK; // 保存ファイルに載っている行
    let stale_row = FIRST_PLAYABLE_TRACK + 1; // 保存ファイルに載っていない行

    // 保存ファイルは行 2 だけ（保存上の track 番号は chord 行のぶんずれる）。
    let save_path = cmrt_history::daw_file_path().unwrap();
    std::fs::create_dir_all(save_path.parent().unwrap()).unwrap();
    std::fs::write(
        &save_path,
        br#"{"tracks":[{"track":1,"meas":[{"meas":1,"mml":"cdef"}]}]}"#,
    )
    .unwrap();

    let stale_wav = crate::cache::cache_wav_path(WorkspaceKind::Persistent, stale_row, 1).unwrap();
    cmrt_core::write_wav(&[0.25, -0.25], 44_100, &stale_wav).unwrap();

    let (mut app, cache_rx) = build_daily_app_with_cache_jobs(&config_app_dir);
    app.workspace_kind = WorkspaceKind::Persistent;
    app.load(TODAY);

    assert_eq!(app.editor.data[kept_row][1], "cdef");
    assert!(app.editor.data[stale_row][1].is_empty());
    assert!(
        app.cache.lock().unwrap()[stale_row][1].state == CacheState::Empty,
        "a row missing from the save file is Empty, so it is never re-rendered"
    );

    app.kick_all_pending();

    let job = cache_rx
        .try_recv()
        .expect("the row that is in the save file is queued");
    assert_eq!((job.track, job.measure), (kept_row, 1));
    assert!(
        matches!(cache_rx.try_recv(), Err(TryRecvError::Empty)),
        "the stale row is not queued"
    );
    assert!(
        stale_wav.is_file(),
        "load_persistent() never touches daw_cache/, so the orphaned wav survives"
    );
}
