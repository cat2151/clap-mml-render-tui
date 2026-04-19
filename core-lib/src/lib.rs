pub use clap_mml_play_server_core::patch_list::{collect_patches, to_relative};
pub use clap_mml_play_server_core::pipeline;
pub use clap_mml_play_server_core::pipeline::{
    ensure_cmrt_dir, ensure_daw_dir, ensure_phrase_dir, mml_render, mml_str_to_smf_bytes,
    mml_to_play, mml_to_smf_bytes, play_samples, write_wav,
};
pub use clap_mml_play_server_core::{host, load_entry, midi, patch_list, render, CoreConfig};

use anyhow::Result;
use clack_host::prelude::PluginEntry;
use mmlabc_to_smf::mml_preprocessor;
use std::sync::{mpsc, OnceLock};

const PATCH_DIR_PREFIXES: [&str; 2] = ["patches_factory", "patches_3rdparty"];
static CACHE_RENDER_PREPARE_QUEUE: OnceLock<CacheRenderPrepareQueue> = OnceLock::new();

mod native_render_probe;

#[cfg(test)]
use native_render_probe::clear_native_render_probe_state_for_tests;
pub use native_render_probe::{
    set_native_probe_logger, NativeProbeLogger, NativeRenderProbeContext,
};
use native_render_probe::{with_native_render_probe, with_requested_native_render_probe};

pub struct CacheRenderInputs {
    patched_cfg: CoreConfig,
    events: Vec<midi::TimedMidiEvent>,
    total_samples: u64,
}

struct CacheRenderPrepareRequest {
    mml: String,
    cfg: CoreConfig,
    response_tx: mpsc::Sender<Result<CacheRenderInputs>>,
}

struct CacheRenderPrepareQueue {
    tx: mpsc::Sender<CacheRenderPrepareRequest>,
}

impl CacheRenderPrepareQueue {
    fn start() -> Self {
        let (tx, rx) = mpsc::channel::<CacheRenderPrepareRequest>();
        std::thread::spawn(move || {
            while let Ok(request) = rx.recv() {
                let result = prepare_cache_render(&request.mml, &request.cfg);
                let _ = request.response_tx.send(result);
            }
        });
        Self { tx }
    }

    fn prepare(&self, mml: &str, cfg: &CoreConfig) -> Result<CacheRenderInputs> {
        let (response_tx, response_rx) = mpsc::channel();
        let request = CacheRenderPrepareRequest {
            mml: mml.to_string(),
            cfg: cfg.clone(),
            response_tx,
        };
        if self.tx.send(request).is_err() {
            return prepare_cache_render(mml, cfg);
        }
        response_rx
            .recv()
            .unwrap_or_else(|_| prepare_cache_render(mml, cfg))
    }
}

fn cache_render_prepare_queue() -> &'static CacheRenderPrepareQueue {
    CACHE_RENDER_PREPARE_QUEUE.get_or_init(CacheRenderPrepareQueue::start)
}

fn prepare_cache_render_via_queue(mml: &str, cfg: &CoreConfig) -> Result<CacheRenderInputs> {
    cache_render_prepare_queue().prepare(mml, cfg)
}

/// DAW モードのプレビューキャッシュ構築専用の MML → レンダリング。
/// - `patch_history.txt` への追記は行わない
/// - DAW 専用の MIDI/WAV キャッシュファイル（`daw_cache.*`）は生成しない
/// - ランダムパッチ選択は無効化し、通常レンダリングと同じ JSON 埋め込みパッチ解決を行う
///
/// # Parameters
/// - `mml`: レンダリング対象の MML 文字列。先頭 JSON によるパッチ指定も受け付ける。
/// - `cfg`: レンダリング設定。JSON にパッチ指定がない場合は `cfg.patch_path` を使う。
/// - `entry`: 読み込み済み CLAP プラグインエントリ。
///
/// # Returns
/// インターリーブされたステレオ PCM サンプル列を `Vec<f32>` で返す。
pub fn mml_render_for_cache(mml: &str, cfg: &CoreConfig, entry: &PluginEntry) -> Result<Vec<f32>> {
    mml_render_for_cache_with_probe(mml, cfg, entry, None)
}

pub fn mml_render_with_probe(
    mml: &str,
    cfg: &CoreConfig,
    entry: &PluginEntry,
    probe_context: Option<&NativeRenderProbeContext>,
) -> Result<(Vec<f32>, String)> {
    let requested_patch_path = requested_patch_path_for_render(mml, cfg);
    with_requested_native_render_probe(probe_context, requested_patch_path.as_deref(), || {
        mml_render(mml, cfg, entry)
    })
}

pub fn mml_render_for_cache_with_probe(
    mml: &str,
    cfg: &CoreConfig,
    entry: &PluginEntry,
    probe_context: Option<&NativeRenderProbeContext>,
) -> Result<Vec<f32>> {
    // MML -> SMF の前処理は単一キューへ流し、render worker 数とは独立して
    // MML 1 本ずつ処理する。
    let prepared = prepare_cache_render_via_queue(mml, cfg)?;
    render_prepared_cache_with_probe(prepared, entry, probe_context)
}

pub fn prepare_cache_render_inputs(mml: &str, cfg: &CoreConfig) -> Result<CacheRenderInputs> {
    prepare_cache_render(mml, cfg)
}

pub fn render_prepared_cache_with_probe(
    prepared: CacheRenderInputs,
    entry: &PluginEntry,
    probe_context: Option<&NativeRenderProbeContext>,
) -> Result<Vec<f32>> {
    let CacheRenderInputs {
        patched_cfg,
        events,
        total_samples,
    } = prepared;
    with_native_render_probe(
        probe_context,
        &patched_cfg,
        events.len(),
        total_samples,
        || render::render_to_memory(&patched_cfg, entry, events, total_samples),
    )
}

/// キャッシュレンダリング前処理を行い、メモリレンダリングに必要な入力を組み立てる。
///
/// 戻り値は `(patched_cfg, events, total_samples)` で、
/// それぞれ「JSON/既定パッチ適用後の設定」「SMF から展開した MIDI イベント列」
/// 「レンダリングすべき総サンプル数」を表す。
fn prepare_cache_render(mml: &str, cfg: &CoreConfig) -> Result<CacheRenderInputs> {
    let preprocessed = mml_preprocessor::extract_embedded_json(mml);
    // 通常レンダリングと同様に、先頭 JSON のパッチ指定を最優先し、
    // 指定がない場合だけ config の既定パッチにフォールバックする。
    let effective_patch = extract_patch_from_json(preprocessed.embedded_json.as_deref(), cfg)
        .or_else(|| cfg.patch_path.clone());

    let smf_bytes = mml_str_to_smf_bytes(&preprocessed.remaining_mml)?;
    let (events, total_samples) = midi::parse_smf_bytes(&smf_bytes, cfg.sample_rate)?;
    let patched_cfg = CoreConfig {
        patch_path: effective_patch,
        random_patch: false,
        ..cfg.clone()
    };

    Ok(CacheRenderInputs {
        patched_cfg,
        events,
        total_samples,
    })
}

fn requested_patch_path_for_render(mml: &str, cfg: &CoreConfig) -> Option<String> {
    let preprocessed = mml_preprocessor::extract_embedded_json(mml);
    extract_patch_from_json(preprocessed.embedded_json.as_deref(), cfg)
        .or_else(|| cfg.patch_path.clone())
}

/// MML 先頭 JSON から `"Surge XT patch"` を取り出してパッチパスに解決する。
///
/// JSON に相対パスが入っていて `cfg.patches_dir` がある場合はその配下のパスに変換し、
/// `cfg.patches_dir` がない場合は JSON の文字列をそのまま返す。
fn extract_patch_from_json(json_str: Option<&str>, cfg: &CoreConfig) -> Option<String> {
    let json_str = json_str?;
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let rel = value.get("Surge XT patch")?.as_str()?;
    let rel_path = normalize_patch_path(rel);

    if let Some(ref base) = cfg.patches_dir {
        let base = std::path::Path::new(base);
        let abs = resolve_patch_path_from_base(base, &rel_path);
        Some(abs.to_string_lossy().into_owned())
    } else {
        Some(rel_path.to_string_lossy().into_owned())
    }
}

fn normalize_patch_path(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(rel).components().collect()
}

fn resolve_patch_path_from_base(
    base: &std::path::Path,
    rel_path: &std::path::Path,
) -> std::path::PathBuf {
    let abs = base.join(rel_path);
    if abs.exists() {
        return abs;
    }

    if rel_path.components().next().is_none() {
        return abs;
    }
    if rel_path
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .is_some_and(|first| {
            PATCH_DIR_PREFIXES
                .iter()
                .any(|prefix| first.eq_ignore_ascii_case(prefix))
        })
    {
        return abs;
    }

    for prefix in PATCH_DIR_PREFIXES {
        let candidate = base.join(prefix).join(rel_path);
        if candidate.exists() {
            return candidate;
        }
    }

    abs
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
