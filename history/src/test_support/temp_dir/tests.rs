use super::*;

const HOUR: Duration = Duration::from_secs(60 * 60);

/// 「今」は epoch からの nanos。小さすぎる値だと連番との区別が付かないので、
/// 判定の下限より後ろに置く。
fn now_nanos_for_test() -> u128 {
    MIN_PLAUSIBLE_NANOS + 10 * HOUR.as_nanos()
}

#[test]
fn unique_test_dir_differs_every_call_and_carries_the_pid() {
    let a = unique_test_dir("same_tag");
    let b = unique_test_dir("same_tag");
    assert_ne!(a, b, "同じ tag でも呼ぶたびに別のパスになること");
    let name = a.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.contains(&format!("_{}_", std::process::id())),
        "pid が入っていない: {name}"
    );
}

#[test]
fn a_temp_dir_guard_removes_its_directory_when_dropped() {
    let path = {
        let tmp = TempDirGuard::new("guard_drop");
        std::fs::create_dir_all(tmp.join("nested")).unwrap();
        std::fs::write(tmp.join("nested").join("f.txt"), b"x").unwrap();
        assert!(tmp.path().exists());
        tmp.path().to_path_buf()
    };
    assert!(!path.exists(), "drop 後も残っている: {}", path.display());
}

#[test]
fn a_fresh_test_dir_is_not_swept() {
    let now = now_nanos_for_test();
    assert!(!is_stale_test_dir(
        &format!("cmrt_test_process_1234_{now}"),
        now,
        HOUR
    ));
}

#[test]
fn a_test_dir_older_than_the_age_limit_is_swept() {
    let now = now_nanos_for_test();
    let created = now - 2 * HOUR.as_nanos();
    assert!(is_stale_test_dir(
        &format!("cmrt_test_process_1234_{created}"),
        now,
        HOUR
    ));
    assert!(is_stale_test_dir(
        &format!("cmrt_test_daw_cache_1234_{created}"),
        now,
        HOUR
    ));
}

/// 末尾が連番のディレクトリを nanos と読むと、**走っている最中のテストのディレクトリを
/// 消す**。実際に `cmrt-notepad` の 1 本をこれで落とした（`{pid}_{連番}` 形式）。
#[test]
fn a_directory_whose_suffix_is_a_counter_is_never_swept() {
    let now = now_nanos_for_test();
    for suffix in [0u128, 1, 7, 1_000] {
        let name = format!("cmrt_test_notepad_history_enter_flush_31337_{suffix}");
        assert!(
            !is_stale_test_dir(&name, now, HOUR),
            "連番を nanos と読んで消している: {name}"
        );
    }
}

#[test]
fn names_outside_the_scheme_are_never_swept() {
    let now = now_nanos_for_test();
    for name in [
        "cmrt_test_history_roundtrip.json".to_string(),
        "cmrt_test_fixed_name".to_string(),
        "some_other_dir".to_string(),
        "cmrt_test_".to_string(),
        // 未来の時刻（時計が巻き戻ったとき）も消さない
        format!("cmrt_test_process_1_{}", now + HOUR.as_nanos()),
    ] {
        assert!(
            !is_stale_test_dir(&name, now, HOUR),
            "消してはいけない: {name}"
        );
    }
}
