//! コード進行カタログを画面へ渡す遅延クロージャ（`chord_chart_catalog_source_from`）。
//!
//! 見るのは「待ったのは初回だけか」と「その待ち時間がログ 1 行に出るか」。

use super::*;

/// カタログの初回待ちは秒数をログへ 1 行だけ残す。体感の許容判断は人間だが、
/// **計測そのものは仕込み済み**であることをここで固定する。
#[test]
fn the_catalog_source_measures_and_logs_only_the_first_wait() {
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let counted = std::sync::Arc::clone(&calls);
    let collected = std::sync::Arc::clone(&lines);
    let source = crate::tui::chord_chart_glue::chord_chart_catalog_source(
        move || {
            counted.fetch_add(1, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_millis(30));
            vec!["I-IV-V-I".to_string(), "I-V-VIm-IV".to_string()]
        },
        move |line| collected.lock().unwrap().push(line.to_string()),
    );
    // 組み立てただけでは引かない（起動時にネットワークを待たない）。
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert!(lines.lock().unwrap().is_empty());

    assert_eq!(source().len(), 2);
    assert_eq!(source().len(), 2);

    assert_eq!(calls.load(Ordering::Relaxed), 2);
    let logged = lines.lock().unwrap().clone();
    assert_eq!(logged.len(), 1, "{logged:?}");
    let line = &logged[0];
    assert!(line.contains("event=catalog-first-load"), "{line}");
    assert!(line.contains("count=2"), "{line}");
    let elapsed: u128 = line
        .split("elapsed_ms=")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("{line}"));
    assert!(elapsed >= 5, "{line}");
}

/// 初回取得の**実際の秒数**を、画面を起動せずに実経路で測る。
///
/// 通常は skip する（ネットワークが要るため）。走らせ方:
///
/// ```text
/// cargo test -p clap-mml-render-tui the_cold_catalog_first_load -- --ignored --nocapture
/// ```
///
/// config ディレクトリを空の temp dir へ逃がすので、キャッシュが無い＝必ず「初回」になる。
/// 測るのは production と同じ [`crate::tui::chord_chart_glue::chord_chart_catalog_source_from`]。
#[test]
#[ignore = "ネットワークが要る（コード進行カタログの初回取得を実測する）"]
fn the_cold_catalog_first_load_is_measured_through_the_real_source() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_chord_chart_cold_catalog_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut cfg = test_config();
    // test_config() は空文字（＝取得しない）なので、実際の既定 URL へ戻す。
    cfg.chord_progression_source = cmrt_runtime::DEFAULT_CHORD_PROGRESSION_SOURCE.to_string();

    let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let collected = std::sync::Arc::clone(&lines);
    let catalog = crate::tui::chord_chart_glue::chord_chart_catalog_source_from(
        crate::chord_progression_source::ChordProgressionSource::spawn(&cfg),
        move |line| collected.lock().unwrap().push(line.to_string()),
    );

    let progressions = catalog();
    let logged = lines.lock().unwrap().clone();
    let line = logged
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("初回取得のログが出ていない: {logged:?}"));
    // 秒数そのものは人間が読む。ここは「測れていること」だけを固定する。
    eprintln!("{line}");
    assert!(line.contains("event=catalog-first-load"), "{line}");
    assert!(
        !progressions.is_empty(),
        "カタログを取得できていない: {line}"
    );

    std::fs::remove_dir_all(&tmp).ok();
}
