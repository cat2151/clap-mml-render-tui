use super::*;

fn utf16_entry(name: &str) -> [u16; 260] {
    let mut buffer = [0u16; 260];
    for (slot, unit) in buffer.iter_mut().zip(name.encode_utf16()) {
        *slot = unit;
    }
    buffer
}

fn named(name: &str, pid: u32) -> ProcessMemory {
    ProcessMemory {
        name: name.to_string(),
        pid,
        working_set_bytes: 0,
    }
}

#[test]
fn the_realtime_play_server_is_a_target() {
    assert!(is_server_executable(&executable_name(&utf16_entry(
        "clap-mml-realtime-play-server.exe"
    ))));
}

#[test]
fn the_render_server_is_a_target() {
    assert!(is_server_executable(&executable_name(&utf16_entry(
        "clap-mml-render-server.exe"
    ))));
}

/// プロセス一覧の exe 名は大文字小文字が環境依存なので、無視して比較する。
#[test]
fn the_comparison_ignores_case() {
    assert!(is_server_executable(&executable_name(&utf16_entry(
        "CLAP-MML-RENDER-SERVER.EXE"
    ))));
}

#[test]
fn unrelated_processes_are_not_targets() {
    assert!(!is_server_executable(&executable_name(&utf16_entry(
        "cmrt.exe"
    ))));
    assert!(!is_server_executable(&executable_name(&utf16_entry(
        "explorer.exe"
    ))));
    // 前方一致で拾わないこと。
    assert!(!is_server_executable(&executable_name(&utf16_entry(
        "clap-mml-render-server.exe.bak"
    ))));
}

/// 埋まっていない末尾は NUL なので、そこで打ち切って名前にする。
#[test]
fn a_name_shorter_than_the_buffer_is_terminated_at_the_nul() {
    let mut buffer = utf16_entry("clap-mml-render-server.exe");
    buffer[200] = u16::from(b'x');

    assert_eq!(
        executable_name(&buffer),
        "clap-mml-render-server.exe".to_string()
    );
}

/// 自プロセス → SERVER_EXECUTABLES の定義順 → pid 昇順。
#[test]
fn the_display_order_puts_the_current_process_first_and_is_value_independent() {
    let own_pid = 42;
    let mut processes = [
        named("clap-mml-render-server.exe", 300),
        named("clap-mml-realtime-play-server.exe", 200),
        named("clap-mml-realtime-play-server.exe", 100),
        named("cmrt.exe", own_pid),
    ];

    processes.sort_by_key(|process| display_order(process, own_pid));

    let order: Vec<(&str, u32)> = processes
        .iter()
        .map(|process| (process.name.as_str(), process.pid))
        .collect();
    assert_eq!(
        order,
        vec![
            ("cmrt.exe", own_pid),
            ("clap-mml-realtime-play-server.exe", 100),
            ("clap-mml-realtime-play-server.exe", 200),
            ("clap-mml-render-server.exe", 300),
        ]
    );
}

/// 一覧に自プロセスが出てこなかったときは補って先頭に置く。
#[test]
fn a_missing_current_process_is_filled_in_at_the_front() {
    let own_pid = 42;
    let mut processes = vec![named("clap-mml-render-server.exe", 300)];

    ensure_self_entry(&mut processes, own_pid);

    assert_eq!(processes.len(), 2);
    assert_eq!(processes[0].pid, own_pid);
    assert!(processes[0].working_set_bytes > 0, "{processes:?}");
}

#[test]
fn an_existing_current_process_is_not_duplicated() {
    let own_pid = 42;
    let mut processes = vec![named("cmrt.exe", own_pid)];

    ensure_self_entry(&mut processes, own_pid);

    assert_eq!(processes.len(), 1);
}

/// 自プロセスは必ず計測できる。
#[test]
fn the_current_process_working_set_is_available() {
    // SAFETY: 疑似ハンドルを返すだけで、失敗しない。
    let working_set = working_set_of(unsafe { GetCurrentProcess() });

    assert!(
        working_set.is_some_and(|bytes| bytes > 0),
        "{working_set:?}"
    );
}

#[test]
fn the_available_physical_memory_is_reported() {
    let available = available_physical_bytes();

    assert!(available.is_some_and(|bytes| bytes > 0), "{available:?}");
}

/// 計測全体が通ること（サーバが起動していなくても自プロセスぶんで成立する）。
#[test]
fn measure_returns_a_snapshot() {
    let snapshot = measure().expect("Windows では計測できるはず");

    assert!(snapshot.total_working_set_bytes > 0, "{snapshot:?}");
    assert!(snapshot.os_available_bytes > 0, "{snapshot:?}");
}

/// 表示上「合計 ≠ 内訳の和」にならないこと。
#[test]
fn the_snapshot_total_is_the_sum_of_its_processes() {
    let snapshot = measure().expect("Windows では計測できるはず");

    // SAFETY: 引数を取らず、失敗しない。
    let own_pid = unsafe { GetCurrentProcessId() };
    assert!(
        snapshot
            .processes
            .iter()
            .any(|process| process.pid == own_pid),
        "{snapshot:?}"
    );

    let sum: u64 = snapshot
        .processes
        .iter()
        .map(|process| process.working_set_bytes)
        .sum();
    assert_eq!(snapshot.total_working_set_bytes, sum, "{snapshot:?}");
}
