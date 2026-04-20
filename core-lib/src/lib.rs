pub use clap_mml_play_server_core::patch_list::{collect_patches, to_relative};
pub use clap_mml_play_server_core::pipeline;
pub use clap_mml_play_server_core::pipeline::{
    ensure_cmrt_dir, ensure_daw_dir, ensure_phrase_dir, mml_str_to_smf_bytes, mml_to_smf_bytes,
    play_samples, write_wav,
};
pub use clap_mml_play_server_core::{host, load_entry, midi, patch_list, render, CoreConfig};

use anyhow::Result;
use clack_host::prelude::PluginEntry;
use mmlabc_to_smf::mml_preprocessor;
use std::{
    sync::{mpsc, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const PATCH_DIR_PREFIXES: [&str; 2] = ["patches_factory", "patches_3rdparty"];
const RENDER_PREROLL_MS: u64 = 100;
const RENDER_CHANNELS: usize = 2;
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

pub fn mml_render(mml: &str, cfg: &CoreConfig, entry: &PluginEntry) -> Result<(Vec<f32>, String)> {
    mml_render_with_probe(mml, cfg, entry, None)
}

pub fn mml_to_play(mml: &str, cfg: &CoreConfig, entry: &PluginEntry) -> Result<String> {
    let (samples, patch_display) = mml_render(mml, cfg, entry)?;
    play_samples(samples, cfg.sample_rate as u32)?;
    Ok(patch_display)
}

// Temporary thin fork of upstream `pipeline::mml_render` so this workspace can
// apply a uniform 100ms preroll workaround to every render path. Remove this
// once `clap-mml-play-server-core` exposes a shared preroll hook/config and
// switch back to re-exporting upstream `mml_render` / `mml_to_play`.
pub fn mml_render_with_probe(
    mml: &str,
    cfg: &CoreConfig,
    entry: &PluginEntry,
    probe_context: Option<&NativeRenderProbeContext>,
) -> Result<(Vec<f32>, String)> {
    let preprocessed = mml_preprocessor::extract_embedded_json(mml);
    let effective_patch =
        resolve_effective_patch_for_full_render(preprocessed.embedded_json.as_deref(), cfg)?;
    append_history(mml, &effective_patch, cfg)?;

    let phrase_dir = ensure_phrase_dir()?;
    let output_midi = phrase_dir.join("output.mid");
    let output_wav = phrase_dir.join("output.wav");
    let output_midi_str = utf8_path_string(&output_midi, "出力MIDIパス")?;
    let output_wav_str = utf8_path_string(&output_wav, "出力WAVパス")?;
    let smf_bytes = mml_str_to_smf_bytes(&preprocessed.remaining_mml)?;
    std::fs::write(&output_midi, &smf_bytes).map_err(|e| {
        anyhow::anyhow!(
            "MIDIファイル書き出し失敗 ({}): {}",
            output_midi.display(),
            e
        )
    })?;
    let (events, total_samples) = parse_events_with_preroll(&smf_bytes, cfg.sample_rate)?;
    let patched_cfg = CoreConfig {
        output_midi: output_midi_str,
        output_wav: output_wav_str.clone(),
        patch_path: effective_patch.clone(),
        ..cfg.clone()
    };
    let requested_patch_path = requested_patch_path_for_render(mml, cfg);
    with_requested_native_render_probe(probe_context, requested_patch_path.as_deref(), || {
        let samples = with_native_render_probe(
            probe_context,
            &patched_cfg,
            events.len(),
            total_samples,
            || render::render_to_memory(&patched_cfg, entry, events.clone(), total_samples),
        )?;
        let samples = trim_render_preroll(samples, preroll_samples(cfg.sample_rate));
        write_wav(&samples, cfg.sample_rate as u32, &output_wav)?;
        Ok((
            samples,
            patch_display_for_render(effective_patch.as_deref(), cfg),
        ))
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
    .map(|samples| trim_render_preroll(samples, preroll_samples(patched_cfg.sample_rate)))
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
    let (events, total_samples) = parse_events_with_preroll(&smf_bytes, cfg.sample_rate)?;
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

fn parse_events_with_preroll(
    smf_bytes: &[u8],
    sample_rate: f64,
) -> Result<(Vec<midi::TimedMidiEvent>, u64)> {
    let (events, total_samples) = midi::parse_smf_bytes(smf_bytes, sample_rate)?;
    Ok(apply_render_preroll(
        events,
        total_samples,
        preroll_samples(sample_rate),
    ))
}

fn preroll_samples(sample_rate: f64) -> u64 {
    ((sample_rate * RENDER_PREROLL_MS as f64) / 1000.0).ceil() as u64
}

fn apply_render_preroll(
    events: Vec<midi::TimedMidiEvent>,
    total_samples: u64,
    preroll_samples: u64,
) -> (Vec<midi::TimedMidiEvent>, u64) {
    if preroll_samples == 0 {
        return (events, total_samples);
    }
    let events = events
        .into_iter()
        .map(|event| midi::TimedMidiEvent {
            sample_pos: event.sample_pos.saturating_add(preroll_samples),
            message: event.message,
        })
        .collect();
    (events, total_samples.saturating_add(preroll_samples))
}

fn trim_render_preroll(samples: Vec<f32>, preroll_samples: u64) -> Vec<f32> {
    let trim_len = preroll_samples as usize * RENDER_CHANNELS;
    samples.into_iter().skip(trim_len).collect()
}

fn resolve_effective_patch_for_full_render(
    embedded_json: Option<&str>,
    cfg: &CoreConfig,
) -> Result<Option<String>> {
    if let Some(patch) = extract_patch_from_json(embedded_json, cfg) {
        return Ok(Some(patch));
    }
    if cfg.random_patch {
        return pick_random_patch(cfg);
    }
    Ok(cfg.patch_path.clone())
}

fn patch_display_for_render(effective_patch: Option<&str>, cfg: &CoreConfig) -> String {
    match effective_patch {
        Some(abs) => {
            if let Some(ref base) = cfg.patches_dir {
                to_relative(base, std::path::Path::new(abs))
            } else {
                abs.to_string()
            }
        }
        None => "(Init Saw)".to_string(),
    }
}

fn pick_random_patch(cfg: &CoreConfig) -> Result<Option<String>> {
    let Some(dir) = &cfg.patches_dir else {
        return Ok(None);
    };
    let patches = collect_patches(dir)?;
    if patches.is_empty() {
        return Ok(None);
    }
    let idx = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or(0) as usize
        % patches.len();
    Ok(Some(patches[idx].to_string_lossy().into_owned()))
}

fn append_history(mml: &str, patch: &Option<String>, cfg: &CoreConfig) -> Result<()> {
    let patch_rel = match patch {
        Some(abs) => patch_display_for_render(Some(abs), cfg),
        None => "(none)".to_string(),
    };
    let preprocessed = mml_preprocessor::extract_embedded_json(mml);
    let mml_body = preprocessed.remaining_mml.trim();
    let line = format!(
        "{{\"Surge XT patch\": \"{}\"}} {}\n",
        patch_rel.replace('\\', "/"),
        mml_body
    );
    let phrase_dir = ensure_phrase_dir()?;
    let Some(base_dir) = phrase_dir.parent() else {
        return Ok(());
    };
    let history_path = base_dir.join("patch_history.txt");
    std::fs::create_dir_all(base_dir)
        .map_err(|e| anyhow::anyhow!("patch_history.txt のディレクトリ作成失敗: {}", e))?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&history_path)
        .map_err(|e| anyhow::anyhow!("patch_history.txt を開けない: {}", e))?;
    file.write_all(line.as_bytes())
        .map_err(|e| anyhow::anyhow!("patch_history.txt への書き込み失敗: {}", e))?;
    Ok(())
}

fn utf8_path_string(path: &std::path::Path, label: &str) -> Result<String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("{label}が非UTF-8です: {}", path.display()))
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
