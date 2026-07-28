//! Windows での実メモリ計測。
//!
//! 自プロセスと clap-mml-play-server の常駐プロセスを、プロセス一覧の 1 回の走査で
//! まとめて拾う。サーバへ問い合わせないので、手動で常駐させたサーバや、
//! 複数起動しているサーバもそのまま拾える。
//!
//! 合計は内訳の総和として組み立てる。自プロセスだけ別経路（`GetCurrentProcess`）で
//! 測ると、走査のタイミング差で「合計 ≠ 内訳の和」になって読み手が混乱するため。

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, PROCESS_ACCESS_RIGHTS,
    PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};

use super::{MemorySnapshot, ProcessMemory};

/// 内訳に出す常駐サーバ。
///
/// 名前の定義元は `cmrt-realtime-play` の `default_realtime_play_server_executable_name()`
/// と `cmrt-offline-render` の render server 実行ファイル名解決だが、どちらも
/// 非公開かつ tui-core からは依存できないため、ここに複製している。
const SERVER_EXECUTABLES: [&str; 2] = [
    "clap-mml-realtime-play-server.exe",
    "clap-mml-render-server.exe",
];

/// プロセス一覧から自プロセスが拾えなかったときに使う表示名。
const UNKNOWN_SELF_NAME: &str = "(self)";

pub(super) fn measure() -> Option<MemorySnapshot> {
    let os_available_bytes = available_physical_bytes()?;
    let processes = collect_processes();
    let total_working_set_bytes = processes
        .iter()
        .map(|process| process.working_set_bytes)
        .fold(0u64, u64::saturating_add);

    Some(MemorySnapshot {
        processes,
        total_working_set_bytes,
        os_available_bytes,
    })
}

fn available_physical_bytes() -> Option<u64> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: status は MEMORYSTATUSEX ぶんの領域を持ち、dwLength にそのサイズを入れてある。
    let succeeded = unsafe { GlobalMemoryStatusEx(&mut status) } != 0;

    succeeded.then_some(status.ullAvailPhys)
}

fn working_set_of(process: HANDLE) -> Option<u64> {
    let size = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: size,
        ..Default::default()
    };
    // SAFETY: process は有効なプロセスハンドル、counters は cb ぶんの領域を持つ。
    let succeeded = unsafe { K32GetProcessMemoryInfo(process, &mut counters, size) } != 0;

    succeeded.then_some(counters.WorkingSetSize as u64)
}

/// 自プロセスと常駐サーバを、表示順に並べて返す。
fn collect_processes() -> Vec<ProcessMemory> {
    // SAFETY: 引数を取らず、失敗しない。
    let own_pid = unsafe { GetCurrentProcessId() };

    let mut processes = scan_processes(own_pid);
    processes.sort_by_key(|process| display_order(process, own_pid));
    // ソート後に入れることで、補った自プロセスが必ず先頭に来る。
    ensure_self_entry(&mut processes, own_pid);

    processes
}

fn scan_processes(own_pid: u32) -> Vec<ProcessMemory> {
    // SAFETY: TH32CS_SNAPPROCESS では第2引数は無視される。
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Vec::new();
    }

    let processes = collect_from_snapshot(snapshot, own_pid);
    // SAFETY: snapshot は CreateToolhelp32Snapshot が返した有効なハンドル。
    unsafe { CloseHandle(snapshot) };

    processes
}

fn collect_from_snapshot(snapshot: HANDLE, own_pid: u32) -> Vec<ProcessMemory> {
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: snapshot は有効なハンドル、entry は dwSize を設定済み。
    if unsafe { Process32FirstW(snapshot, &mut entry) } == 0 {
        return Vec::new();
    }

    let mut processes = Vec::new();
    loop {
        let name = executable_name(&entry.szExeFile);
        if entry.th32ProcessID == own_pid || is_server_executable(&name) {
            processes.push(ProcessMemory {
                name,
                pid: entry.th32ProcessID,
                working_set_bytes: working_set_of_pid(entry.th32ProcessID),
            });
        }
        // SAFETY: 直前の Process32FirstW / Process32NextW が埋めた entry を再利用する。
        if unsafe { Process32NextW(snapshot, &mut entry) } == 0 {
            break;
        }
    }

    processes
}

/// 表示順のキー。自プロセス、[`SERVER_EXECUTABLES`] の定義順、同名内は pid 昇順。
///
/// Working Set の降順にすると、1秒ごとの再計測で値が拮抗した行が入れ替わって
/// ちらつくため、値に依存しないキーで固定する。
fn display_order(process: &ProcessMemory, own_pid: u32) -> (usize, u32) {
    if process.pid == own_pid {
        return (0, process.pid);
    }

    let rank = SERVER_EXECUTABLES
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(&process.name))
        .unwrap_or(SERVER_EXECUTABLES.len());

    (rank + 1, process.pid)
}

/// プロセス一覧に自プロセスが出てこなかったときの保険。
fn ensure_self_entry(processes: &mut Vec<ProcessMemory>, own_pid: u32) {
    if processes.iter().any(|process| process.pid == own_pid) {
        return;
    }

    // SAFETY: 疑似ハンドルを返すだけで、失敗しない。CloseHandle も不要。
    let working_set_bytes = working_set_of(unsafe { GetCurrentProcess() }).unwrap_or(0);
    processes.insert(
        0,
        ProcessMemory {
            name: own_executable_name(),
            pid: own_pid,
            working_set_bytes,
        },
    );
}

fn own_executable_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| UNKNOWN_SELF_NAME.to_string())
}

/// `PROCESSENTRY32W::szExeFile` の UTF-16 バッファを文字列にする。
///
/// 埋まっていない末尾は NUL なので、そこで打ち切る。
fn executable_name(exe_file: &[u16]) -> String {
    let end = exe_file
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(exe_file.len());

    String::from_utf16_lossy(&exe_file[..end])
}

fn is_server_executable(name: &str) -> bool {
    SERVER_EXECUTABLES
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(name))
}

fn working_set_of_pid(pid: u32) -> u64 {
    // 通常は LIMITED で足りるが、取れない環境向けに従来の権限でも一度試す。
    const ACCESS_CANDIDATES: [PROCESS_ACCESS_RIGHTS; 2] = [
        PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
    ];

    for access in ACCESS_CANDIDATES {
        // SAFETY: pid はプロセス一覧から得たもの。失敗時は null が返る。
        let process = unsafe { OpenProcess(access, 0, pid) };
        if process.is_null() {
            continue;
        }

        let working_set = working_set_of(process);
        // SAFETY: process は OpenProcess が返した有効なハンドル。
        unsafe { CloseHandle(process) };

        if let Some(bytes) = working_set {
            return bytes;
        }
    }

    // 権限が無いプロセスは計測全体を落とさず 0 として飛ばす。
    0
}

#[cfg(test)]
mod tests;
