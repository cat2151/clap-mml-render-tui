use std::{
    ffi::OsString,
    path::PathBuf,
    sync::{Mutex, MutexGuard, OnceLock},
};

use super::*;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(Mutex::default)
}

/// `ensure_cmrt_dir` の基点を一時ディレクトリへ寄せる。
///
/// これを挟まないと、キャッシュ置き場を触るテストが実ユーザーの
/// config ディレクトリを読み書きしてしまう（掃除のテストは実際にファイルを消す）。
/// `CMRT_BASE_DIR` はプロセス全体の環境変数なので、Mutex で直列化する。
struct BaseDirGuard {
    _lock: MutexGuard<'static, ()>,
    previous: Option<OsString>,
    dir: PathBuf,
}

impl BaseDirGuard {
    fn new(name: &str) -> Self {
        let lock = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = std::env::temp_dir().join(format!(
            "cmrt-cache-dirs-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let previous = std::env::var_os("CMRT_BASE_DIR");
        std::env::set_var("CMRT_BASE_DIR", &dir);
        Self {
            _lock: lock,
            previous,
            dir,
        }
    }
}

impl Drop for BaseDirGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(previous) => std::env::set_var("CMRT_BASE_DIR", previous),
            None => std::env::remove_var("CMRT_BASE_DIR"),
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn namespace_uses_plugin_file_stem() {
    assert_eq!(
        cache_plugin_namespace(
            r"C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap"
        ),
        "Surge XT"
    );
    assert_eq!(
        cache_plugin_namespace("/Library/Audio/Plug-Ins/CLAP/Dexed.clap"),
        "Dexed"
    );
}

#[test]
fn namespace_ignores_surrounding_whitespace_in_the_resolved_path() {
    // config の表現ではなく解決後の plugin_path だけでキャッシュ置き場が決まる。
    let from_profile = cmrt_runtime::default_plugin_path();
    assert_eq!(
        cache_plugin_namespace(from_profile),
        cache_plugin_namespace(&format!("  {from_profile}  "))
    );
}

#[test]
fn namespace_differs_between_surge_and_dexed() {
    assert_ne!(
        cache_plugin_namespace(cmrt_runtime::default_plugin_path()),
        cache_plugin_namespace(cmrt_runtime::default_dexed_plugin_path())
    );
}

/// 3 つめのプラグインも自分の名前空間を持つ。
///
/// キャッシュキーは MML 文字列の hash なので、**音色を指定していない行**は
/// プラグインを替えても同じキーになる。ここが分かれていないと、既定プラグインを
/// Vaporizer2 へ替えたときに Surge / Dexed が書いた WAV を読んでしまう。
/// 名前空間は `plugin_path` の stem なので無改修で分かれるはずで、これはその番人。
#[test]
fn namespace_differs_for_vaporizer2_too() {
    let vaporizer2 = cache_plugin_namespace(cmrt_runtime::default_vaporizer2_plugin_path());

    assert_ne!(
        vaporizer2,
        cache_plugin_namespace(cmrt_runtime::default_plugin_path())
    );
    assert_ne!(
        vaporizer2,
        cache_plugin_namespace(cmrt_runtime::default_dexed_plugin_path())
    );
    // ディレクトリ名として使えない文字が残っていないこと。
    assert!(!vaporizer2.contains(|c: char| std::path::is_separator(c) || c == ':'));
    assert_ne!(vaporizer2, DEFAULT_CACHE_PLUGIN_NAMESPACE);
}

#[test]
fn namespace_replaces_characters_that_cannot_be_a_directory_name() {
    // file_stem は最後のパス区切りより後ろを見るので、区切り以外の危険文字だけが残る。
    assert_eq!(cache_plugin_namespace("my*plugin?.clap"), "my_plugin_");
}

#[test]
fn namespace_falls_back_when_nothing_usable_remains() {
    assert_eq!(cache_plugin_namespace(""), DEFAULT_CACHE_PLUGIN_NAMESPACE);
    assert_eq!(
        cache_plugin_namespace("   "),
        DEFAULT_CACHE_PLUGIN_NAMESPACE
    );
    // 末尾のドットだけの名前は Windows がディレクトリ名として受け付けない。
    assert_eq!(
        cache_plugin_namespace("..."),
        DEFAULT_CACHE_PLUGIN_NAMESPACE
    );
}

#[test]
fn cache_dirs_are_created_under_the_plugin_namespace() {
    let _base = BaseDirGuard::new("namespace");
    let dir = ensure_daw_cache_dir().unwrap();
    assert!(dir.is_dir());
    assert_eq!(dir.file_name().unwrap().to_str().unwrap(), namespace());
    assert_eq!(dir.parent().unwrap(), ensure_daw_dir().unwrap());

    let dir = ensure_notepad_cache_dir().unwrap();
    assert!(dir.is_dir());
    assert_eq!(dir.file_name().unwrap().to_str().unwrap(), namespace());
    assert_eq!(
        dir.parent().unwrap(),
        ensure_cmrt_dir().unwrap().join("notepad_cache")
    );
}

#[test]
fn legacy_cache_file_names_are_recognized() {
    assert!(is_legacy_daw_cell_wav("track0_meas1"));
    assert!(is_legacy_daw_cell_wav("track12_meas345"));
    assert!(!is_legacy_daw_cell_wav("daw_cache"));
    assert!(!is_legacy_daw_cell_wav("track_meas1"));
    assert!(!is_legacy_daw_cell_wav("trackA_meas1"));
    assert!(!is_legacy_daw_cell_wav("track0_meas"));

    assert!(is_legacy_notepad_cache_wav("0123456789abcdef"));
    assert!(!is_legacy_notepad_cache_wav("0123456789abcde"));
    assert!(!is_legacy_notepad_cache_wav("0123456789abcdeg"));
    assert!(!is_legacy_notepad_cache_wav("daw_cache"));
}

#[test]
fn legacy_cleanup_keeps_render_server_intermediate_files() {
    let _base = BaseDirGuard::new("legacy-cleanup");
    let daw_dir = ensure_daw_dir().unwrap();
    let cell = daw_dir.join("track7_meas9.wav");
    let intermediate = daw_dir.join("daw_cache.wav");
    let midi = daw_dir.join("daw_cache.mid");
    for path in [&cell, &intermediate, &midi] {
        std::fs::write(path, b"x").unwrap();
    }
    let notepad_dir = ensure_cmrt_dir().unwrap().join("notepad_cache");
    std::fs::create_dir_all(&notepad_dir).unwrap();
    let legacy = notepad_dir.join("00112233445566ff.wav");
    std::fs::write(&legacy, b"x").unwrap();
    // 名前空間ディレクトリの中身は移行対象ではないので残る。
    let namespaced = ensure_notepad_cache_dir()
        .unwrap()
        .join("00112233445566ff.wav");
    std::fs::write(&namespaced, b"x").unwrap();

    remove_legacy_unnamespaced_caches();

    assert!(!cell.exists(), "旧セル WAV が残っている");
    assert!(!legacy.exists(), "旧 notepad キャッシュが残っている");
    assert!(
        intermediate.exists(),
        "render-server の中間 WAV を消してはいけない"
    );
    assert!(
        midi.exists(),
        "render-server の中間 MIDI を消してはいけない"
    );
    assert!(
        namespaced.exists(),
        "名前空間内のキャッシュを消してはいけない"
    );
}
