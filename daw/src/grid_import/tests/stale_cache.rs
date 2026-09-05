//! **Grid の 1 周で Daily DAW を全置換したときに、前の曲のキャッシュ WAV が
//! どうなるか**を固定する（`docs/adr/0018-page-replacement-clears-the-cache.md`）。
//!
//! rollover とまったく同型の穴がここにもあった。陳腐化は 2 段で成立する。
//!
//! 1. **消さない** … `apply_project_snapshot_state()`（`daw/src/project.rs`）は
//!    メモリ上のキャッシュを [`CacheState::Empty`] へ落とすだけで、
//!    `daw_cache/<plugin>/daily/` のファイルには一切触らなかった
//! 2. **上書きもされない** … 直後の `kick_all_pending()` は**中身のあるセルしか**
//!    投入しないので、置換で消えた行・小節の WAV は焼き直されない
//!
//! ファイル名は `track{行}_meas{小節}.wav` で**日付も hash も入らない**うえ、
//! 演奏ループはファイルの存在しか見ない（Stage 1 で確定）。だから前の曲の WAV が
//! 今日のファイル名を占めたまま鳴り続ける。**rollover と違い、これは日中に起きる。**
//!
//! ここに並ぶのは「直したあとの正しい振る舞い」で、
//! `clear_daily_cache_after_full_replacement()` を外すと赤くなる向きに書いてある。
//! **共通の底へ動かされないための番人は
//! `daily::tests::stale_cache::a_failed_rollover_keeps_yesterdays_page_and_its_cache_wav`**
//! （底へ置くと Resume / rollover 失敗が、復元するはずの WAV を自分で消す）。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::*;
use crate::input::tests::temp_local_dirs;
use crate::CacheState;

const TODAY: &str = "2026-09-04";

/// 全置換 import を受け取る側の Daily DAW。
///
/// `config_app_dir` を渡すのは `save()`（= `save_daily_recovery()`）を
/// 本当に書かせるため。書かれた `current.json` は
/// `scripts/check_daw_cache_staleness.py` がそのまま読める。
fn daily_app(app_dir: &Path) -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = crate::input::tests::build_test_app();
    app.workspace_kind = WorkspaceKind::Daily;
    app.config_app_dir = Some(app_dir.to_path_buf());
    app.daily_page_date = Some(TODAY.to_owned());
    (app, cache_rx)
}

/// 実機と同じ形（`track2..8_meas1..5` の 35 ファイル）の「前の曲」を焼いておく。
///
/// 長さを 2 種類にしてあるのは、実データが 09-02 世代と 09-03 世代の混在だったから
/// （ADR 0018 の実測）。`check_daw_cache_staleness.py` はこの状態を
/// 「長さが 2 種類ある」で NG と言う。
fn write_previous_songs_cache(daily_dir: &Path) -> Vec<PathBuf> {
    let mut written = Vec::new();
    for row in FIRST_PLAYABLE_TRACK..FIRST_PLAYABLE_TRACK + 7 {
        for measure in 1..=5 {
            let path = daily_dir.join(format!("track{row}_meas{measure}.wav"));
            let samples = vec![0.1_f32; if measure == 5 { 96 } else { 64 }];
            cmrt_core::write_wav(&samples, 44_100, &path).unwrap();
            written.push(path);
        }
    }
    written
}

fn wav_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".wav"))
        .collect();
    names.sort();
    names
}

/// 掃除のログ行（`scripts/daw_log_sent_rows.py` が読むのと同じ行）。
fn cache_cleared_log_line(app: &DawApp) -> Option<String> {
    let log = app.log_lines.lock().unwrap();
    log.iter()
        .find(|line| line.contains("grid import cache cleared: "))
        .cloned()
}

/// **Stage 8 の本体。** 全置換すると、前の曲のキャッシュ WAV はディスクから消える。
///
/// 消さないと、置換で空になった行・小節の名前を前の曲のファイルが占めたままになり、
/// 演奏ループ（ファイルの存在しか見ない）がそれを鳴らす。
/// `clear_daily_cache_after_full_replacement()` を外すと `left` が 35 件のまま赤くなる。
#[test]
fn a_grid_import_deletes_the_previous_songs_cache_wavs_from_disk() {
    let (temp, _env_guard) = temp_local_dirs("grid_import_stale_delete");
    let daily_dir = crate::cache::ensure_workspace_cache_dir(WorkspaceKind::Daily).unwrap();
    let before = write_previous_songs_cache(&daily_dir);
    assert_eq!(before.len(), 35, "the fixture must match the real cache");
    assert_eq!(wav_names(&daily_dir).len(), 35, "and be on disk first");
    let keep_me = daily_dir.join("notes.txt");
    std::fs::write(&keep_me, b"not a wav").unwrap();

    let (mut app, _cache_rx) = daily_app(&temp.path().join("clap-mml-render-tui"));
    // 置換後は 2 行 x 2 小節。前の曲の 7 行 x 5 小節のうち大半が行き場を失う。
    app.replace_with_grid_song(song()).unwrap();

    let left = wav_names(&daily_dir);
    assert!(
        left.is_empty(),
        "a full replacement must not leave the previous song's wavs behind; left={left:?}"
    );
    assert!(
        keep_me.is_file(),
        "only *.wav is swept, so anything else in the directory survives"
    );
    let cleared = cache_cleared_log_line(&app)
        .expect("the cleanup must leave a machine-checkable line in the log");
    assert!(
        cleared.contains("removed=35 wav"),
        "the log must say how many files went away; line={cleared}"
    );
    assert!(
        cleared.contains("daily"),
        "and which directory was swept (a wrong namespace would show up here); line={cleared}"
    );
}

/// 掃除したぶんのうち、**新しい曲に在る行・小節は焼き直される**。
///
/// 「消して終わり」だと今度は音が出ない。`replace_with_grid_song()` の
/// `kick_all_pending()` が、置換後のセルを 1 つ残らず投入することを受け口で見る。
/// 前の曲にしか無い行（row4 以降）と小節（meas3 以降）は**投入されない**
/// ＝ もう鳴らない。これが陳腐化の 2 段目で、直っていないのは変わらない
/// （変わったのは「上書きされるのを待っている古い WAV」が居なくなったこと）。
#[test]
fn a_grid_import_re_renders_every_cell_of_the_new_song_and_nothing_else() {
    let (temp, _env_guard) = temp_local_dirs("grid_import_stale_rerender");
    let daily_dir = crate::cache::ensure_workspace_cache_dir(WorkspaceKind::Daily).unwrap();
    write_previous_songs_cache(&daily_dir);

    let (mut app, cache_rx) = daily_app(&temp.path().join("clap-mml-render-tui"));
    app.replace_with_grid_song(song()).unwrap();

    let queued: BTreeSet<(usize, usize)> = cache_rx
        .try_iter()
        .map(|job| (job.track, job.measure))
        .collect();
    let expected: BTreeSet<(usize, usize)> = [
        (FIRST_PLAYABLE_TRACK, 1),
        (FIRST_PLAYABLE_TRACK, 2),
        (FIRST_PLAYABLE_TRACK + 1, 1),
        (FIRST_PLAYABLE_TRACK + 1, 2),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        queued, expected,
        "every cell of the new song is queued, and no cell of the old one is"
    );
    assert!(
        app.cache.lock().unwrap()[FIRST_PLAYABLE_TRACK][1].state == CacheState::Rendering,
        "the queued cell is marked Rendering, so the empty result above is not vacuous"
    );
}

/// **番人。** 掃除がワークスペースの判定より前へ動かされていないこと。
///
/// `replace_with_grid_song()` は Persistent DAW では最初に `bail!` する。
/// 掃除をその手前へ出すと、Persistent の画面から Daily のキャッシュを消してしまう。
#[test]
fn a_rejected_import_into_a_persistent_daw_touches_no_cache_wav() {
    let (_temp, _env_guard) = temp_local_dirs("grid_import_stale_persistent");
    let daily_dir = crate::cache::ensure_workspace_cache_dir(WorkspaceKind::Daily).unwrap();
    write_previous_songs_cache(&daily_dir);

    let (mut app, _cache_rx) = crate::input::tests::build_test_app();
    assert_eq!(app.workspace_kind, WorkspaceKind::Persistent);
    let error = app.replace_with_grid_song(song()).unwrap_err();

    assert!(error.to_string().contains("Daily DAW"));
    assert_eq!(
        wav_names(&daily_dir).len(),
        35,
        "a rejected import must not sweep the daily cache"
    );
    assert!(
        cache_cleared_log_line(&app).is_none(),
        "and must not claim it did"
    );
}

/// **`scripts/check_daw_cache_staleness.py` に、全置換の前後を判定させる**
/// （通常は skip。判定スクリプトのパスを env で渡したときだけ走る）。
///
/// ```text
/// CMRT_CACHE_STALENESS_SCRIPT=scripts/check_daw_cache_staleness.py \
///   cargo test -p cmrt-daw --lib grid_import::tests::stale_cache
/// ```
///
/// 単体テストの assert は「WAV が消えたか」までしか見ない。
/// **ユーザーの実データを判定したのと同じスクリプトに同じ形の状態を食わせて、
/// 置換前 exit 1 → 置換後 exit 0 が出ることまで見る**のがこのテスト。
/// exit code が両方向へ動くので、「いつでも緑」でも「いつでも赤」でもない。
///
/// 実サーバーは要らない。render worker が書く WAV
/// （`init::cache_worker::store_cache_job_samples`）は、投入された job の
/// ファイル名と小節長からそのまま作れる。
#[test]
fn a_grid_import_leaves_a_daily_cache_that_the_staleness_script_calls_clean() {
    let Ok(script) = std::env::var("CMRT_CACHE_STALENESS_SCRIPT") else {
        eprintln!(
            "skip: CMRT_CACHE_STALENESS_SCRIPT にスクリプトのパスを渡すと \
             check_daw_cache_staleness.py で判定する"
        );
        return;
    };
    let python = std::env::var("CMRT_PYTHON").unwrap_or_else(|_| "python".to_owned());
    let (temp, _env_guard) = temp_local_dirs("grid_import_stale_script");
    let app_dir = temp.path().join("clap-mml-render-tui");
    let daily_dir = crate::cache::ensure_workspace_cache_dir(WorkspaceKind::Daily).unwrap();
    write_previous_songs_cache(&daily_dir);

    let (mut app, cache_rx) = daily_app(&app_dir);
    // 置換前の状態（前の曲の 35 ファイル・長さ 2 種）を project ごと書き出す。
    app.save();
    let project = crate::daily::daily_current_path(&app_dir);
    assert!(project.is_file(), "current.json must be written first");
    let before = run_staleness_script(&python, &script, &daily_dir, &project);
    assert_eq!(
        before, 1,
        "the previous song's cache must be judged NG（世代の混在＋project に無い行）"
    );

    app.replace_with_grid_song(song()).unwrap();
    // render worker が焼き直すぶんを、投入された job のとおりに作る。
    for job in cache_rx.try_iter() {
        let path = daily_dir.join(format!("track{}_meas{}.wav", job.track, job.measure));
        let samples = vec![0.1_f32; job.measure_samples];
        cmrt_core::write_wav(&samples, 44_100, &path).unwrap();
    }

    let after = run_staleness_script(&python, &script, &daily_dir, &project);
    assert_eq!(
        after, 0,
        "after a full replacement the daily cache holds exactly one generation, \
         and every wav belongs to a row of today's project"
    );
}

fn run_staleness_script(python: &str, script: &str, dir: &Path, project: &Path) -> i32 {
    let output = std::process::Command::new(python)
        .arg(script)
        .arg("--dir")
        .arg(dir)
        .arg("--project")
        .arg(project)
        .output()
        .expect("python と check_daw_cache_staleness.py が要る");
    eprintln!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    output.status.code().expect("スクリプトは exit code を返す")
}
