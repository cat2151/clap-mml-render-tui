pub use clap_mml_play_server_core::patch_list::{collect_patches, to_relative};
pub use clap_mml_play_server_core::pipeline;
pub use clap_mml_play_server_core::pipeline::{
    ensure_cmrt_dir, ensure_daw_dir, ensure_phrase_dir, mml_str_to_smf_bytes, mml_to_smf_bytes,
    play_samples, write_wav, RenderOptions, RenderPreroll,
};
pub use clap_mml_play_server_core::{host, load_entry, midi, patch_list, render, CoreConfig};

use anyhow::Result;
use clack_host::prelude::PluginEntry;
use mmlabc_to_smf::mml_preprocessor;
use std::{
    borrow::Cow,
    sync::{mpsc, OnceLock},
};

const PATCH_DIR_PREFIXES: [&str; 2] = ["patches_factory", "patches_3rdparty"];
const RENDER_PREROLL_MS: u64 = 100;
static CACHE_RENDER_PREPARE_QUEUE: OnceLock<CacheRenderPrepareQueue> = OnceLock::new();

mod native_render_probe;

#[cfg(test)]
use native_render_probe::clear_native_render_probe_state_for_tests;
#[cfg(test)]
use native_render_probe::with_native_render_probe;
use native_render_probe::with_requested_native_render_probe;
pub use native_render_probe::{
    set_native_probe_logger, NativeProbeLogger, NativeRenderProbeContext,
};

pub struct CacheRenderInputs {
    mml: String,
    cfg: CoreConfig,
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
/// - `patch_history.txt` への追記は core 側で行わない
/// - ランダムパッチ選択は core 側で無効化する
/// - 100ms preroll は core 側の `RenderOptions` で指定する
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

pub fn mml_render(mml: &str, cfg: &CoreConfig, entry: &PluginEntry) -> Result<(Vec<f32>, String)> {
    mml_render_with_probe(mml, cfg, entry, None)
}

pub fn mml_to_play(mml: &str, cfg: &CoreConfig, entry: &PluginEntry) -> Result<String> {
    let (samples, patch_display) = mml_render(mml, cfg, entry)?;
    play_samples(samples, cfg.sample_rate as u32)?;
    Ok(patch_display)
}

pub fn mml_render_with_probe(
    mml: &str,
    cfg: &CoreConfig,
    entry: &PluginEntry,
    probe_context: Option<&NativeRenderProbeContext>,
) -> Result<(Vec<f32>, String)> {
    let mml = mml_with_resolved_embedded_patch(mml, cfg);
    let requested_patch_path = requested_patch_path_for_render(mml.as_ref(), cfg);
    with_requested_native_render_probe(probe_context, requested_patch_path.as_deref(), || {
        pipeline::mml_render_with_options(mml.as_ref(), cfg, entry, render_options())
    })
}

pub fn mml_render_for_cache_with_probe(
    mml: &str,
    cfg: &CoreConfig,
    entry: &PluginEntry,
    probe_context: Option<&NativeRenderProbeContext>,
) -> Result<Vec<f32>> {
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
    let CacheRenderInputs { mml, cfg } = prepared;
    let requested_patch_path = requested_patch_path_for_render(&mml, &cfg);
    with_requested_native_render_probe(probe_context, requested_patch_path.as_deref(), || {
        pipeline::mml_render_for_cache_with_options(&mml, &cfg, entry, render_options())
    })
}

/// キャッシュレンダリングの事前入力を組み立てる。
/// preroll の適用はレンダー時に core 側へ `RenderOptions` として渡す。
fn prepare_cache_render(mml: &str, cfg: &CoreConfig) -> Result<CacheRenderInputs> {
    Ok(CacheRenderInputs {
        mml: mml_with_resolved_embedded_patch(mml, cfg).into_owned(),
        cfg: cfg.clone(),
    })
}

fn render_options() -> RenderOptions {
    RenderOptions::new().with_preroll_ms(RENDER_PREROLL_MS)
}

fn requested_patch_path_for_render(mml: &str, cfg: &CoreConfig) -> Option<String> {
    let preprocessed = mml_preprocessor::extract_embedded_json(mml);
    extract_patch_from_json(preprocessed.embedded_json.as_deref(), cfg)
        .or_else(|| cfg.patch_path.clone())
}

fn mml_with_resolved_embedded_patch<'a>(mml: &'a str, cfg: &CoreConfig) -> Cow<'a, str> {
    let preprocessed = mml_preprocessor::extract_embedded_json(mml);
    let Some(resolved_patch) = extract_patch_from_json(preprocessed.embedded_json.as_deref(), cfg)
    else {
        return Cow::Borrowed(mml);
    };
    let Some(embedded_json) = preprocessed.embedded_json.as_deref() else {
        return Cow::Borrowed(mml);
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(embedded_json) else {
        return Cow::Borrowed(mml);
    };
    let Some(object) = value.as_object_mut() else {
        return Cow::Borrowed(mml);
    };
    object.insert(
        "Surge XT patch".to_string(),
        serde_json::Value::String(patch_value_for_core_embedded_json(&resolved_patch, cfg)),
    );
    match serde_json::to_string(&value) {
        Ok(json) => Cow::Owned(format!("{json}{}", preprocessed.remaining_mml)),
        Err(_) => Cow::Borrowed(mml),
    }
}

fn patch_value_for_core_embedded_json(resolved_patch: &str, cfg: &CoreConfig) -> String {
    let resolved_path = std::path::Path::new(resolved_patch);
    if let Some(base) = cfg.patches_dir.as_deref() {
        if let Ok(relative_path) = resolved_path.strip_prefix(std::path::Path::new(base)) {
            return relative_path.to_string_lossy().into_owned();
        }
    }
    resolved_patch.to_string()
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
