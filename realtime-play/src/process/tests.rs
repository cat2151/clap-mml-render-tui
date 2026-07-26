use super::*;

#[test]
fn executable_lookup_returns_the_existing_full_path() {
    let directory = std::env::temp_dir().join(format!(
        "cmrt_realtime_server_path_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let executable = default_realtime_play_server_executable_name();
    let expected = directory.join(executable);
    std::fs::write(&expected, []).unwrap();

    let resolved = executable_in_paths(executable, [directory.clone()].into_iter());

    assert_eq!(resolved, Some(expected.clone()));
    std::fs::remove_file(expected).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn startup_progress_parser_accepts_only_valid_instance_counts() {
    assert_eq!(
        parse_server_startup_progress("cmrt-server-startup: instances=7/16"),
        Some((7, 16))
    );
    assert_eq!(
        parse_server_startup_progress("cmrt-server-startup: instances=17/16"),
        None
    );
    assert_eq!(parse_server_startup_progress("unrelated output"), None);
}
