use super::*;

fn utf16_entry(name: &str) -> [u16; 260] {
    let mut buffer = [0u16; 260];
    for (slot, unit) in buffer.iter_mut().zip(name.encode_utf16()) {
        *slot = unit;
    }
    buffer
}

#[test]
fn the_realtime_play_server_is_a_target() {
    assert!(is_target_executable(&utf16_entry(
        "clap-mml-realtime-play-server.exe"
    )));
}

#[test]
fn the_render_server_is_a_target() {
    assert!(is_target_executable(&utf16_entry(
        "clap-mml-render-server.exe"
    )));
}

/// プロセス一覧の exe 名は大文字小文字が環境依存なので、無視して比較する。
#[test]
fn the_comparison_ignores_case() {
    assert!(is_target_executable(&utf16_entry(
        "CLAP-MML-RENDER-SERVER.EXE"
    )));
}

#[test]
fn unrelated_processes_are_not_targets() {
    assert!(!is_target_executable(&utf16_entry("cmrt.exe")));
    assert!(!is_target_executable(&utf16_entry("explorer.exe")));
    // 前方一致で拾わないこと。
    assert!(!is_target_executable(&utf16_entry(
        "clap-mml-render-server.exe.bak"
    )));
}

/// 埋まっていない末尾は NUL なので、そこで打ち切って比較する。
#[test]
fn a_name_shorter_than_the_buffer_is_terminated_at_the_nul() {
    let mut buffer = utf16_entry("clap-mml-render-server.exe");
    buffer[200] = u16::from(b'x');

    assert!(is_target_executable(&buffer));
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
