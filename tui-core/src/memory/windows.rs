//! Windows での実メモリ計測。
//!
//! cmrt 本体は自プロセスのハンドルから、clap-mml-play-server の常駐プロセスは
//! プロセス一覧を exe 名で走査して合算する。サーバへ問い合わせないので、
//! 手動で常駐させたサーバや、複数起動しているサーバもそのまま拾える。

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, PROCESS_ACCESS_RIGHTS, PROCESS_QUERY_INFORMATION,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};

use super::MemorySnapshot;

/// 合算対象の常駐サーバ。
///
/// 名前の定義元は `cmrt-realtime-play` の `default_realtime_play_server_executable_name()`
/// と `cmrt-offline-render` の render server 実行ファイル名解決だが、どちらも
/// 非公開かつ tui-core からは依存できないため、ここに複製している。
const SERVER_EXECUTABLES: [&str; 2] = [
    "clap-mml-realtime-play-server.exe",
    "clap-mml-render-server.exe",
];

pub(super) fn measure() -> Option<MemorySnapshot> {
    let os_available_bytes = available_physical_bytes()?;
    // SAFETY: 疑似ハンドルを返すだけで、失敗しない。CloseHandle も不要。
    let own_working_set = working_set_of(unsafe { GetCurrentProcess() })?;

    Some(MemorySnapshot {
        total_working_set_bytes: own_working_set.saturating_add(server_working_set_total()),
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

fn server_working_set_total() -> u64 {
    // SAFETY: TH32CS_SNAPPROCESS では第2引数は無視される。
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return 0;
    }

    let total = sum_server_working_sets(snapshot);
    // SAFETY: snapshot は CreateToolhelp32Snapshot が返した有効なハンドル。
    unsafe { CloseHandle(snapshot) };

    total
}

fn sum_server_working_sets(snapshot: HANDLE) -> u64 {
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: snapshot は有効なハンドル、entry は dwSize を設定済み。
    if unsafe { Process32FirstW(snapshot, &mut entry) } == 0 {
        return 0;
    }

    let mut total = 0u64;
    loop {
        if is_target_executable(&entry.szExeFile) {
            total = total.saturating_add(working_set_of_pid(entry.th32ProcessID));
        }
        // SAFETY: 直前の Process32FirstW / Process32NextW が埋めた entry を再利用する。
        if unsafe { Process32NextW(snapshot, &mut entry) } == 0 {
            break;
        }
    }

    total
}

fn is_target_executable(exe_file: &[u16]) -> bool {
    let end = exe_file
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(exe_file.len());
    let name = String::from_utf16_lossy(&exe_file[..end]);

    SERVER_EXECUTABLES
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&name))
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
