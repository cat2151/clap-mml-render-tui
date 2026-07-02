//! notepad (非DAW) モード用のディスク WAV キャッシュ。
//!
//! DAW モードの per-cell WAV キャッシュ（`app/src/daw/cache.rs`）と異なり、
//! notepad モードのキャッシュキーは track/measure のような位置情報ではなく
//! 「現在行の trim 済みテキスト」そのものなので、MML ハッシュ値をファイル名に
//! そのまま使う content-addressable 方式にする。同一内容には常に同一パスが
//! 対応するため、DAW の `DawSessionState.cached_measures` のような
//! 別建てのハッシュ索引ファイルは不要（ファイルの存在自体が妥当性の証拠になる）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// ディスクキャッシュに保持する WAV ファイルの上限件数。
/// メモリ側の `AUDIO_CACHE_MAX_ENTRIES` と同じ値を使う（性質は別のリソースだが、
/// 「直近使ったフレーズを一定件数だけ保持する」という意図は共通のため）。
pub(super) const NOTEPAD_DISK_CACHE_MAX_FILES: usize = super::AUDIO_CACHE_MAX_ENTRIES;

fn notepad_cache_wav_path(mml: &str) -> Option<PathBuf> {
    let dir = cmrt_core::ensure_notepad_cache_dir().ok()?;
    Some(dir.join(format!(
        "{:016x}.wav",
        crate::history::daw_cache_mml_hash(mml)
    )))
}

/// 現在行の内容に対応する妥当なディスクキャッシュ WAV があれば読み込んで返す。
/// sample_rate / channels が現在の設定と一致しない場合や、ファイルが無い・壊れている
/// 場合は `None` を返し、呼び出し元は通常のレンダリング経路にフォールバックする。
///
/// ヒット時はファイルの mtime を更新し、再起動をまたいだ LRU 順序（＝最後に
/// 使われた順）をファイルシステム上に反映する。
pub(super) fn load_valid_cached_wav(mml: &str, expected_sample_rate: u32) -> Option<Vec<f32>> {
    let path = notepad_cache_wav_path(mml)?;
    let info = crate::wav_io::read_wav_cache_info(&path).ok()?;
    if info.spec.sample_rate != expected_sample_rate || info.spec.channels != 2 {
        return None;
    }
    let samples = crate::wav_io::load_wav_samples(&path).ok()?;
    if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&path) {
        let _ = file.set_modified(std::time::SystemTime::now());
    }
    Some(samples)
}

/// `audio_cache` の全エントリをディスクへ書き出す。既に同一ハッシュのファイルが
/// 存在する場合（＝内容が同一）は書き直さない。書き込み後、上限件数を超えた分は
/// mtime が最も古いファイルから削除する。
pub(super) fn flush_audio_cache_to_disk(cache: &HashMap<String, Vec<f32>>, sample_rate: u32) {
    let Ok(dir) = cmrt_core::ensure_notepad_cache_dir() else {
        return;
    };
    for (mml, samples) in cache.iter() {
        let path = dir.join(format!(
            "{:016x}.wav",
            crate::history::daw_cache_mml_hash(mml)
        ));
        if path.exists() {
            continue;
        }
        let _ = cmrt_core::write_wav(samples, sample_rate, &path);
    }
    enforce_disk_cache_file_cap(&dir, NOTEPAD_DISK_CACHE_MAX_FILES);
}

fn enforce_disk_cache_file_cap(dir: &Path, max_files: usize) {
    let mut entries: Vec<(PathBuf, std::time::SystemTime)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("wav"))
        .filter_map(|entry| {
            entry
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .map(|mtime| (entry.path(), mtime))
        })
        .collect();
    if entries.len() <= max_files {
        return;
    }
    entries.sort_by_key(|(_, mtime)| *mtime);
    let remove_count = entries.len() - max_files;
    for (path, _) in entries.into_iter().take(remove_count) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn isolated_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cmrt_test_notepad_disk_cache_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// テストプロセス全体を汚さないよう `CMRT_BASE_DIR` を一時ディレクトリへ
    /// リダイレクトする（`crate::test_utils::set_local_dir_envs` と同じ仕組み）。
    fn isolated_cmrt_base_dir_env(name: &str) -> (PathBuf, crate::test_utils::TestEnvGuard) {
        let base = isolated_test_dir(name);
        let guard = crate::test_utils::set_local_dir_envs(&base);
        (base, guard)
    }

    fn write_test_wav(path: &Path, sample_rate: u32, channels: u16, samples: &[f32]) {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for sample in samples {
            writer.write_sample(*sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn hash_is_stable_and_distinguishes_embedded_json_prefix() {
        let a = crate::history::daw_cache_mml_hash("cde");
        let b = crate::history::daw_cache_mml_hash("cde");
        assert_eq!(a, b);

        let c = crate::history::daw_cache_mml_hash(r#"{"Surge XT patch": "x"}cde"#);
        assert_ne!(a, c);
    }

    #[test]
    fn load_valid_cached_wav_returns_none_when_file_absent() {
        let (_base, _guard) = isolated_cmrt_base_dir_env("miss");
        assert!(load_valid_cached_wav("cde", 48_000).is_none());
    }

    #[test]
    fn load_valid_cached_wav_rejects_stale_sample_rate() {
        let (_base, _guard) = isolated_cmrt_base_dir_env("stale_rate");
        let path = notepad_cache_wav_path("cde").unwrap();
        write_test_wav(&path, 44_100, 2, &[0.0, 0.0, 0.1, 0.1]);
        assert!(load_valid_cached_wav("cde", 48_000).is_none());
    }

    #[test]
    fn load_valid_cached_wav_reads_back_matching_samples() {
        let (_base, _guard) = isolated_cmrt_base_dir_env("hit");
        let samples = vec![0.0_f32, 0.0, 0.25, -0.25];
        let path = notepad_cache_wav_path("cde").unwrap();
        write_test_wav(&path, 48_000, 2, &samples);

        let loaded = load_valid_cached_wav("cde", 48_000).unwrap();
        assert_eq!(loaded, samples);
    }

    #[test]
    fn load_valid_cached_wav_returns_none_for_corrupt_file() {
        let (_base, _guard) = isolated_cmrt_base_dir_env("corrupt");
        let path = notepad_cache_wav_path("cde").unwrap();
        std::fs::write(&path, b"not a wav file").unwrap();
        assert!(load_valid_cached_wav("cde", 48_000).is_none());
    }

    #[test]
    fn load_valid_cached_wav_touches_mtime_on_hit() {
        let (_base, _guard) = isolated_cmrt_base_dir_env("touch_mtime");
        let path = notepad_cache_wav_path("cde").unwrap();
        write_test_wav(&path, 48_000, 2, &[0.0, 0.0]);
        let old_mtime = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(old_mtime)
            .unwrap();

        assert!(load_valid_cached_wav("cde", 48_000).is_some());

        let new_mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert!(new_mtime > old_mtime);
    }

    #[test]
    fn enforce_disk_cache_file_cap_removes_oldest_files_first() {
        let dir = isolated_test_dir("cap");
        let now = std::time::SystemTime::now();
        for i in 0..5u64 {
            let path = dir.join(format!("{i}.wav"));
            write_test_wav(&path, 48_000, 2, &[0.0, 0.0]);
            let mtime = now - std::time::Duration::from_secs(5 - i);
            std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(mtime)
                .unwrap();
        }

        enforce_disk_cache_file_cap(&dir, 2);

        let remaining: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.contains(&"3.wav".to_string()));
        assert!(remaining.contains(&"4.wav".to_string()));
    }

    #[test]
    fn flush_audio_cache_to_disk_writes_one_file_per_entry() {
        let (_base, _guard) = isolated_cmrt_base_dir_env("flush_basic");
        let mut cache = HashMap::new();
        cache.insert("cde".to_string(), vec![0.0_f32, 0.0, 0.1, 0.1]);
        cache.insert("efg".to_string(), vec![0.2_f32, 0.2]);

        flush_audio_cache_to_disk(&cache, 48_000);

        assert_eq!(
            load_valid_cached_wav("cde", 48_000).unwrap(),
            vec![0.0, 0.0, 0.1, 0.1]
        );
        assert_eq!(load_valid_cached_wav("efg", 48_000).unwrap(), vec![0.2, 0.2]);
    }

    #[test]
    fn flush_audio_cache_to_disk_skips_rewriting_existing_file() {
        let (_base, _guard) = isolated_cmrt_base_dir_env("skip_rewrite");
        let mut cache = HashMap::new();
        cache.insert("cde".to_string(), vec![0.0_f32, 0.0]);
        flush_audio_cache_to_disk(&cache, 48_000);

        let path = notepad_cache_wav_path("cde").unwrap();
        let old_mtime = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(old_mtime)
            .unwrap();

        // 同じ内容（同じハッシュ）で再度 flush しても、既存ファイルは書き直されない。
        flush_audio_cache_to_disk(&cache, 48_000);
        let mtime_after = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(mtime_after, old_mtime);
    }

    #[test]
    fn flush_audio_cache_to_disk_enforces_file_cap() {
        let (_base, _guard) = isolated_cmrt_base_dir_env("flush_cap");
        let dir = cmrt_core::ensure_notepad_cache_dir().unwrap();
        let mut cache = HashMap::new();
        for i in 0..(NOTEPAD_DISK_CACHE_MAX_FILES + 3) {
            cache.insert(format!("mml{i}"), vec![0.0_f32, 0.0]);
        }

        flush_audio_cache_to_disk(&cache, 48_000);

        let count = std::fs::read_dir(&dir).unwrap().flatten().count();
        assert_eq!(count, NOTEPAD_DISK_CACHE_MAX_FILES);
    }
}
