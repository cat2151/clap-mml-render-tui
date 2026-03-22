use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Process-wide lock for tests that mutate environment variables.
pub(crate) fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) struct TestEnvGuard {
    _lock: MutexGuard<'static, ()>,
    vars: Vec<(&'static str, Option<String>)>,
}

impl TestEnvGuard {
    pub(crate) fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
        let lock = env_lock()
            .lock()
            .expect("test environment lock should not be poisoned");
        Self {
            _lock: lock,
            vars: set_env_vars([(key, value)]),
        }
    }
}

impl Drop for TestEnvGuard {
    fn drop(&mut self) {
        for (key, original) in self.vars.iter().rev() {
            match original {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// Redirects OS-specific data-directory environment variables to a test-only path.
pub(crate) fn set_data_local_dir_envs(base: &Path) -> TestEnvGuard {
    let lock = env_lock()
        .lock()
        .expect("test environment lock should not be poisoned");
    #[cfg(unix)]
    {
        let xdg_data_home = base.join("xdg-data");
        let home = base.join("home");
        std::fs::create_dir_all(&xdg_data_home).ok();
        std::fs::create_dir_all(&home).ok();
        return TestEnvGuard {
            _lock: lock,
            vars: set_env_vars([("XDG_DATA_HOME", &xdg_data_home), ("HOME", &home)]),
        };
    }
    #[cfg(windows)]
    {
        let local_app_data = base.join("LocalAppData");
        let app_data = base.join("AppData");
        let user_profile = base.join("UserProfile");
        std::fs::create_dir_all(&local_app_data).ok();
        std::fs::create_dir_all(&app_data).ok();
        std::fs::create_dir_all(&user_profile).ok();
        return TestEnvGuard {
            _lock: lock,
            vars: set_env_vars([
                ("LOCALAPPDATA", &local_app_data),
                ("APPDATA", &app_data),
                ("USERPROFILE", &user_profile),
            ]),
        };
    }
    #[cfg(not(any(unix, windows)))]
    {
        let vars = Vec::new();
        return TestEnvGuard { _lock: lock, vars };
    }
}

/// Builds the expected history.json path using the same production resolver as history.rs.
pub(crate) fn session_state_path_for_test() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("clap-mml-render-tui").join("history.json"))
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
