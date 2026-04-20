use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use ratatui::buffer::Buffer;

/// Process-wide lock for tests that mutate environment variables.
pub(crate) fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
fn default_test_app_dir_path() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let unique = format!(
            "cmrt_test_process_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch in tests")
                .as_nanos()
        );
        let app_dir = std::env::temp_dir()
            .join(unique)
            .join("clap-mml-render-tui");
        std::fs::create_dir_all(app_dir.join("history")).ok();
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::env::set_var("CMRT_BASE_DIR", &app_dir);
        app_dir
    })
}

#[cfg(test)]
pub(crate) fn default_test_app_dir() -> Option<PathBuf> {
    Some(default_test_app_dir_path().clone())
}

#[cfg(not(test))]
pub(crate) fn default_test_app_dir() -> Option<PathBuf> {
    None
}

pub(crate) fn test_app_dir_for_current_thread_or_default() -> Option<PathBuf> {
    crate::history::test_history_app_dir_for_current_thread().or_else(default_test_app_dir)
}

pub(crate) struct TestEnvGuard {
    _lock: Option<MutexGuard<'static, ()>>,
    vars: Vec<(&'static str, Option<String>)>,
    previous_history_app_dir: Option<Option<PathBuf>>,
}

impl Drop for TestEnvGuard {
    fn drop(&mut self) {
        for (key, original) in self.vars.iter().rev() {
            match original {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        if let Some(previous_history_app_dir) = self.previous_history_app_dir.take() {
            crate::history::set_test_history_app_dir_for_current_thread(previous_history_app_dir);
        }
    }
}

/// Redirects test persistence to a dedicated app dir for the current thread.
/// This isolates history/config/log paths and the core-lib output base together.
pub(crate) fn set_local_dir_envs(base: &Path) -> TestEnvGuard {
    let app_dir = base.join("clap-mml-render-tui");
    let history_dir = app_dir.join("history");
    let lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::fs::create_dir_all(&history_dir).ok();
    let previous_history_app_dir =
        crate::history::set_test_history_app_dir_for_current_thread(Some(app_dir.clone()));
    TestEnvGuard {
        _lock: Some(lock),
        vars: set_env_vars([("CMRT_BASE_DIR", &app_dir)]),
        previous_history_app_dir: Some(previous_history_app_dir),
    }
}

/// Backward-compatible wrapper for older tests using the previous helper name.
#[allow(dead_code)]
pub(crate) fn set_data_local_dir_envs(base: &Path) -> TestEnvGuard {
    set_local_dir_envs(base)
}

/// Builds the expected history.json path using the same production resolver as history.rs.
pub(crate) fn session_state_path_for_test() -> Option<PathBuf> {
    test_app_dir_for_current_thread_or_default()
        .map(|d| d.join("history").join("history.json"))
        .or_else(|| {
            dirs::config_local_dir().map(|d| {
                d.join("clap-mml-render-tui")
                    .join("history")
                    .join("history.json")
            })
        })
}

pub(crate) fn legacy_session_state_path_for_test() -> Option<PathBuf> {
    test_app_dir_for_current_thread_or_default()
        .map(|d| d.join("history.json"))
        .or_else(|| {
            dirs::data_local_dir().map(|d| d.join("clap-mml-render-tui").join("history.json"))
        })
}

pub(crate) fn legacy_daw_session_state_path_for_test() -> Option<PathBuf> {
    test_app_dir_for_current_thread_or_default()
        .map(|d| d.join("history_daw.json"))
        .or_else(|| {
            dirs::data_local_dir().map(|d| d.join("clap-mml-render-tui").join("history_daw.json"))
        })
}

pub(crate) fn legacy_patch_phrase_store_path_for_test() -> Option<PathBuf> {
    test_app_dir_for_current_thread_or_default()
        .map(|d| d.join("patch_history.json"))
        .or_else(|| {
            dirs::data_local_dir().map(|d| d.join("clap-mml-render-tui").join("patch_history.json"))
        })
}

pub(crate) fn legacy_daw_file_path_for_test() -> Option<PathBuf> {
    test_app_dir_for_current_thread_or_default()
        .map(|d| d.join("daw.json"))
        .or_else(|| dirs::data_local_dir().map(|d| d.join("clap-mml-render-tui").join("daw.json")))
}

pub(crate) fn patch_phrase_store_path_for_test() -> Option<PathBuf> {
    test_app_dir_for_current_thread_or_default()
        .map(|d| d.join("history").join("patch_history.json"))
        .or_else(|| {
            dirs::config_local_dir().map(|d| {
                d.join("clap-mml-render-tui")
                    .join("history")
                    .join("patch_history.json")
            })
        })
}

fn set_env_vars<'a, I, V>(vars: I) -> Vec<(&'static str, Option<String>)>
where
    I: IntoIterator<Item = (&'static str, V)>,
    V: AsRef<OsStr> + 'a,
{
    vars.into_iter()
        .map(|(key, value)| {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            (key, original)
        })
        .collect()
}

pub(crate) fn find_text_ignoring_spaces(buffer: &Buffer, text: &str) -> (u16, u16) {
    for y in 0..buffer.area.height {
        let mut normalized = String::new();
        let mut x_positions = Vec::new();
        for x in 0..buffer.area.width {
            let symbol = buffer
                .cell((x, y))
                .unwrap_or_else(|| panic!("failed to access buffer cell at ({x}, {y})"))
                .symbol();
            if symbol == " " || symbol.is_empty() {
                continue;
            }
            for ch in symbol.chars() {
                normalized.push(ch);
                x_positions.push(x);
            }
        }
        if let Some(byte_index) = normalized.find(text) {
            let char_index = normalized[..byte_index].chars().count();
            return (x_positions[char_index], y);
        }
    }
    panic!("text not found in buffer when ignoring spaces: {text}");
}

pub(crate) fn help_overlay_bounds(buffer: &Buffer) -> (u16, u16, u16, u16) {
    let (title_x, top) = find_text_ignoring_spaces(buffer, "ヘルプ(Keybinds)");

    let mut left = title_x;
    while left > 0
        && buffer
            .cell((left, top))
            .unwrap_or_else(|| panic!("failed to access buffer cell at ({left}, {top})"))
            .symbol()
            != "┌"
    {
        left -= 1;
    }

    let mut right = title_x;
    while right + 1 < buffer.area.width
        && buffer
            .cell((right, top))
            .unwrap_or_else(|| panic!("failed to access buffer cell at ({right}, {top})"))
            .symbol()
            != "┐"
    {
        right += 1;
    }

    let mut bottom = top;
    while bottom + 1 < buffer.area.height {
        if buffer
            .cell((left, bottom))
            .unwrap_or_else(|| panic!("failed to access buffer cell at ({left}, {bottom})"))
            .symbol()
            == "└"
            && buffer
                .cell((right, bottom))
                .unwrap_or_else(|| panic!("failed to access buffer cell at ({right}, {bottom})"))
                .symbol()
                == "┘"
        {
            break;
        }
        bottom += 1;
    }

    (left, top, right, bottom)
}
