//! レンダリング結果キャッシュの置き場を、使用中プラグインごとに分ける。
//!
//! キャッシュキーは `cmrt_history::daw_cache_mml_hash` ＝ MML 文字列そのものの hash
//! なので、音色を指定していない行は Surge と Dexed で同じキーになる。キーへ
//! プラグインを混ぜるには 2 repo 合わせて 40 箇所ある hash 呼び出し全部へ引数を
//! 足すことになり、`DawCachedMeasure::normalize()` のように config を持てない
//! 場所も含む。そこで**ディレクトリを分ける**ことで誤ヒットを断つ。
//!
//! **音色を指定した行は、カタログに複数プラグインが並んでも衝突しない。** MML 先頭の
//! JSON に patch の表示パスが埋まっており、Surge の `.fxp` パスと Dexed の `.syx/NN`
//! パスでは MML 文字列そのものが違うため。
//!
//! 名前空間は解決済み `plugin_path` のファイル名（拡張子なし）1 本で決める。
//! profile 名や `plugin_id` ではなく実体のファイル名を使うので、同じ CLAP なら
//! `[plugins."Surge XT"]` で場所を上書きしても一貫した名前になる。
//!
//! **名前空間が 1 つでよい根拠は「1 プロセス 1 プラグイン」ではない**（カタログには
//! 複数プラグインの音色が並ぶ）。**音色を無指定にした行が鳴るプラグインは既定
//! プラグイン 1 つに固定されている**こと（`docs/adr/0004-default-plugin-owns-unspecified-patches.md`）が根拠。
//! 衝突しうるのはその無指定の行だけなので、起動時に一度だけ
//! [`init_cache_plugin_namespace`] で決めれば足りる。

use std::{path::PathBuf, sync::OnceLock};

use anyhow::Result;

use crate::{ensure_cmrt_dir, ensure_daw_dir};

const DAW_CACHE_DIR_NAME: &str = "daw_cache";
const NOTEPAD_CACHE_DIR_NAME: &str = "notepad_cache";

/// 名前空間が決まっていないときに使う名前。
///
/// 実運用では main が起動直後に [`init_cache_plugin_namespace`] を呼ぶので現れない。
/// テストや、プラグインを持たない補助バイナリ向けの落とし所。
pub const DEFAULT_CACHE_PLUGIN_NAMESPACE: &str = "unknown-plugin";

static CACHE_PLUGIN_NAMESPACE: OnceLock<String> = OnceLock::new();

/// 使用中プラグインの `plugin_path` からキャッシュ名前空間を決める。
///
/// ここへ渡すのは**既定プラグイン**の `plugin_path`。2 回目以降の呼び出しは無視する
/// （名前空間を分ける必要があるのは音色無指定の行だけで、それは既定プラグインで鳴る）。
pub fn init_cache_plugin_namespace(plugin_path: &str) {
    let _ = CACHE_PLUGIN_NAMESPACE.set(cache_plugin_namespace(plugin_path));
}

fn namespace() -> &'static str {
    CACHE_PLUGIN_NAMESPACE
        .get()
        .map(String::as_str)
        .unwrap_or(DEFAULT_CACHE_PLUGIN_NAMESPACE)
}

/// `plugin_path` からディレクトリ名 1 つ分の名前空間を作る。
///
/// ファイル名から拡張子を落とし、ディレクトリ名として安全でない文字を `_` へ潰す。
/// 何も残らなければ [`DEFAULT_CACHE_PLUGIN_NAMESPACE`]。
pub fn cache_plugin_namespace(plugin_path: &str) -> String {
    let sanitized = cmrt_runtime::plugin_file_stem(plugin_path)
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect::<String>();
    // Windows は末尾の `.` と空白を無視するため、そのままディレクトリ名にできない。
    let trimmed = sanitized.trim().trim_end_matches('.').trim();
    if trimmed.is_empty() {
        DEFAULT_CACHE_PLUGIN_NAMESPACE.to_string()
    } else {
        trimmed.to_string()
    }
}

/// DAW モードのセル WAV キャッシュ（`track{t}_meas{m}.wav`）置き場。
///
/// render-server が中間ファイル（`daw_cache.mid` / `daw_cache.wav`）に使う
/// [`ensure_daw_dir`] とは別階層になる。セル WAV を読み書きするのは TUI プロセス
/// だけなので、TUI 側だけ名前空間を切っても整合は崩れない。
pub fn ensure_daw_cache_dir() -> Result<PathBuf> {
    let dir = ensure_cmrt_dir()?
        .join(DAW_CACHE_DIR_NAME)
        .join(namespace());
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// notepad (非DAW) モードの行単位オーディオキャッシュ置き場。
/// `ensure_daw_dir` / `ensure_phrase_dir` と同じ `ensure_cmrt_dir` 配下に置く。
pub fn ensure_notepad_cache_dir() -> Result<PathBuf> {
    let dir = ensure_cmrt_dir()?
        .join(NOTEPAD_CACHE_DIR_NAME)
        .join(namespace());
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 旧バージョンが残したキャッシュを、現在の配置へ 1 回だけ移行・掃除する。
///
/// `daw/<plugin>/` にある名前空間付きセル WAV は `daw_cache/<plugin>/` へ移す。
/// 名前空間を切る前のキャッシュは再利用できないため、ファイル名がキャッシュ専用の
/// 形をしているものだけを消す。いずれも
/// render-server が `daw/` 直下に置く中間ファイル（`daw_cache.mid` / `daw_cache.wav`）
/// には触れない。移行後は旧配置が空になるので、次回以降は空振りで終わる。
pub fn migrate_legacy_caches() {
    if let Ok(dir) = ensure_daw_dir() {
        remove_files_in(&dir, is_legacy_daw_cell_wav);
        if let Ok(cache_root) = ensure_cmrt_dir().map(|dir| dir.join(DAW_CACHE_DIR_NAME)) {
            migrate_namespaced_daw_caches(&dir, &cache_root);
        }
    }
    if let Ok(dir) = ensure_cmrt_dir() {
        remove_files_in(
            &dir.join(NOTEPAD_CACHE_DIR_NAME),
            is_legacy_notepad_cache_wav,
        );
    }
}

fn migrate_namespaced_daw_caches(legacy_root: &std::path::Path, cache_root: &std::path::Path) {
    let Ok(namespaces) = std::fs::read_dir(legacy_root) else {
        return;
    };
    for namespace in namespaces.flatten() {
        let legacy_dir = namespace.path();
        if !namespace
            .file_type()
            .is_ok_and(|file_type| file_type.is_dir())
        {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&legacy_dir) else {
            continue;
        };
        let destination_dir = cache_root.join(namespace.file_name());
        for file in files.flatten() {
            let source = file.path();
            if !file.file_type().is_ok_and(|file_type| file_type.is_file())
                || !is_cache_file(&source, is_legacy_daw_cell_wav)
            {
                continue;
            }
            if std::fs::create_dir_all(&destination_dir).is_err() {
                break;
            }
            let destination = destination_dir.join(file.file_name());
            if destination.is_file() {
                // 新配置に同じセルのキャッシュがあれば、そちらを優先する。
                let _ = std::fs::remove_file(&source);
            } else if !destination.exists() {
                let _ = std::fs::rename(&source, destination);
            }
        }
        // キャッシュ以外のファイルがあれば失敗してそのまま残る。
        let _ = std::fs::remove_dir(&legacy_dir);
    }
}

/// DAW のセル WAV は `track{t}_meas{m}.wav` という形しか作らない。
fn is_legacy_daw_cell_wav(stem: &str) -> bool {
    let Some(rest) = stem.strip_prefix("track") else {
        return false;
    };
    let Some((track, measure)) = rest.split_once("_meas") else {
        return false;
    };
    !track.is_empty()
        && !measure.is_empty()
        && track.bytes().all(|b| b.is_ascii_digit())
        && measure.bytes().all(|b| b.is_ascii_digit())
}

/// notepad のキャッシュは MML ハッシュそのもの（`{hash:016x}.wav`）。
fn is_legacy_notepad_cache_wav(stem: &str) -> bool {
    stem.len() == 16 && stem.bytes().all(|b| b.is_ascii_hexdigit())
}

fn remove_files_in(dir: &std::path::Path, is_target: fn(&str) -> bool) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if is_cache_file(&path, is_target) {
            let _ = std::fs::remove_file(&path);
        }
    }
}

fn is_cache_file(path: &std::path::Path, is_target: fn(&str) -> bool) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("wav")
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(is_target)
}

#[cfg(test)]
mod tests;
