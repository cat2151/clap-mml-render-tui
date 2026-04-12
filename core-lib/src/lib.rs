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
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, OnceLock},
};

const PATCH_DIR_PREFIXES: [&str; 2] = ["patches_factory", "patches_3rdparty"];
static CACHE_RENDER_PREPARE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub type NativeProbeLogger = Arc<dyn Fn(&str) + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeRenderCallerKind {
    CacheWorker,
    PlaybackCurrent,
    PlaybackLookahead,
    Preview,
    PreviewPrefetch,
    TuiPlayback,
    TuiPrefetch,
}

impl NativeRenderCallerKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::CacheWorker => "cache_worker",
            Self::PlaybackCurrent => "playback_current",
            Self::PlaybackLookahead => "playback_lookahead",
            Self::Preview => "preview",
            Self::PreviewPrefetch => "preview_prefetch",
            Self::TuiPlayback => "tui_playback",
            Self::TuiPrefetch => "tui_prefetch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NativeRenderProbeDetails {
    CacheWorker {
        track: usize,
        measure: usize,
        generation: u64,
        rendered_mml_hash: u64,
    },
    TrackRender {
        track: usize,
        measure_index: usize,
        active_track_count: usize,
        snapshot_hash: u64,
    },
    TuiRender {
        session: Option<u64>,
        active_render_count: usize,
        snapshot_hash: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NativeRenderSnapshotKey {
    CacheWorker {
        track: usize,
        measure: usize,
        generation: u64,
        rendered_mml_hash: u64,
    },
    TrackRender {
        track: usize,
        measure_index: usize,
        snapshot_hash: u64,
    },
    TuiRender {
        snapshot_hash: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRenderProbeContext {
    caller_kind: NativeRenderCallerKind,
    offline_render_workers: usize,
    details: NativeRenderProbeDetails,
}

impl NativeRenderProbeContext {
    pub fn cache_worker(
        track: usize,
        measure: usize,
        generation: u64,
        rendered_mml_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::CacheWorker,
            offline_render_workers,
            details: NativeRenderProbeDetails::CacheWorker {
                track,
                measure,
                generation,
                rendered_mml_hash,
            },
        }
    }

    pub fn playback_current(
        track: usize,
        measure_index: usize,
        active_track_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::PlaybackCurrent,
            offline_render_workers,
            details: NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                active_track_count,
                snapshot_hash,
            },
        }
    }

    pub fn playback_lookahead(
        track: usize,
        measure_index: usize,
        active_track_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::PlaybackLookahead,
            offline_render_workers,
            details: NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                active_track_count,
                snapshot_hash,
            },
        }
    }

    pub fn preview(
        track: usize,
        measure_index: usize,
        active_track_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::Preview,
            offline_render_workers,
            details: NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                active_track_count,
                snapshot_hash,
            },
        }
    }

    pub fn preview_prefetch(
        track: usize,
        measure_index: usize,
        active_track_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::PreviewPrefetch,
            offline_render_workers,
            details: NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                active_track_count,
                snapshot_hash,
            },
        }
    }

    pub fn tui_playback(
        session: u64,
        active_render_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::TuiPlayback,
            offline_render_workers,
            details: NativeRenderProbeDetails::TuiRender {
                session: Some(session),
                active_render_count,
                snapshot_hash,
            },
        }
    }

    pub fn tui_prefetch(
        active_render_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::TuiPrefetch,
            offline_render_workers,
            details: NativeRenderProbeDetails::TuiRender {
                session: None,
                active_render_count,
                snapshot_hash,
            },
        }
    }

    fn snapshot_key(&self) -> NativeRenderSnapshotKey {
        match &self.details {
            NativeRenderProbeDetails::CacheWorker {
                track,
                measure,
                generation,
                rendered_mml_hash,
            } => NativeRenderSnapshotKey::CacheWorker {
                track: *track,
                measure: *measure,
                generation: *generation,
                rendered_mml_hash: *rendered_mml_hash,
            },
            NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                snapshot_hash,
                ..
            } => NativeRenderSnapshotKey::TrackRender {
                track: *track,
                measure_index: *measure_index,
                snapshot_hash: *snapshot_hash,
            },
            NativeRenderProbeDetails::TuiRender { snapshot_hash, .. } => {
                NativeRenderSnapshotKey::TuiRender {
                    snapshot_hash: *snapshot_hash,
                }
            }
        }
    }

    fn format_fields(&self) -> String {
        match &self.details {
            NativeRenderProbeDetails::CacheWorker {
                track,
                measure,
                generation,
                rendered_mml_hash,
            } => format!(
                "caller={} workers={} track={} measure={} generation={} rendered_mml_hash={}",
                self.caller_kind.as_str(),
                self.offline_render_workers,
                track,
                measure,
                generation,
                format_u64_hex(*rendered_mml_hash),
            ),
            NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                active_track_count,
                snapshot_hash,
            } => format!(
                "caller={} workers={} track={} measure_index={} meas={} active_tracks={} snapshot_hash={}",
                self.caller_kind.as_str(),
                self.offline_render_workers,
                track,
                measure_index,
                measure_index + 1,
                active_track_count,
                format_u64_hex(*snapshot_hash),
            ),
            NativeRenderProbeDetails::TuiRender {
                session,
                active_render_count,
                snapshot_hash,
            } => match session {
                Some(session) => format!(
                    "caller={} workers={} session={} active_renders={} snapshot_hash={}",
                    self.caller_kind.as_str(),
                    self.offline_render_workers,
                    session,
                    active_render_count,
                    format_u64_hex(*snapshot_hash),
                ),
                None => format!(
                    "caller={} workers={} active_renders={} snapshot_hash={}",
                    self.caller_kind.as_str(),
                    self.offline_render_workers,
                    active_render_count,
                    format_u64_hex(*snapshot_hash),
                ),
            },
        }
    }
}

#[derive(Default)]
struct NativeRenderProbeState {
    next_probe_id: u64,
    in_flight: BTreeMap<u64, NativeRenderProbeContext>,
}

struct NativeRenderProbeDecision {
    probe_id: u64,
    should_probe: bool,
    overlap_count: usize,
    overlap_callers: String,
    same_snapshot_overlap: bool,
    mixed_callers_overlap: bool,
}

struct NativeRenderProbeGuard {
    context: NativeRenderProbeContext,
    decision: NativeRenderProbeDecision,
    returned: bool,
}

impl NativeRenderProbeGuard {
    fn enter_with_before_line<F>(context: &NativeRenderProbeContext, build_before_line: F) -> Self
    where
        F: FnOnce(&NativeRenderProbeContext, &NativeRenderProbeDecision) -> String,
    {
        let decision = begin_native_render_probe(context);
        if decision.should_probe {
            emit_native_probe_log(build_before_line(context, &decision));
        }
        Self {
            context: context.clone(),
            decision,
            returned: false,
        }
    }

    fn enter_prepared(
        context: &NativeRenderProbeContext,
        patched_cfg: &CoreConfig,
        event_count: usize,
        total_samples: u64,
    ) -> Self {
        Self::enter_with_before_line(context, |context, decision| {
            format_native_probe_before_line(
                context,
                decision,
                patched_cfg,
                event_count,
                total_samples,
            )
        })
    }

    fn enter_requested(
        context: &NativeRenderProbeContext,
        requested_patch_path: Option<&str>,
    ) -> Self {
        Self::enter_with_before_line(context, |context, decision| {
            format_requested_native_probe_before_line(context, decision, requested_patch_path)
        })
    }

    fn mark_returned(&mut self) {
        self.returned = true;
    }
}

impl Drop for NativeRenderProbeGuard {
    fn drop(&mut self) {
        end_native_render_probe(self.decision.probe_id);
        if self.returned && self.decision.should_probe {
            emit_native_probe_log(format_native_probe_after_line(
                &self.context,
                &self.decision,
            ));
        }
    }
}

fn native_render_probe_state() -> &'static Mutex<NativeRenderProbeState> {
    static STATE: OnceLock<Mutex<NativeRenderProbeState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(NativeRenderProbeState::default()))
}

fn native_probe_logger() -> &'static Mutex<Option<NativeProbeLogger>> {
    static LOGGER: OnceLock<Mutex<Option<NativeProbeLogger>>> = OnceLock::new();
    LOGGER.get_or_init(|| Mutex::new(None))
}

pub fn set_native_probe_logger(logger: Option<NativeProbeLogger>) {
    *native_probe_logger()
        .lock()
        .expect("native probe logger lock should not be poisoned") = logger;
}

fn next_native_probe_id(state: &mut NativeRenderProbeState) -> u64 {
    state.next_probe_id = state.next_probe_id.wrapping_add(1);
    if state.next_probe_id == 0 {
        state.next_probe_id = 1;
    }
    state.next_probe_id
}

fn begin_native_render_probe(context: &NativeRenderProbeContext) -> NativeRenderProbeDecision {
    let mut state = native_render_probe_state()
        .lock()
        .expect("native render probe state lock should not be poisoned");
    let overlapping: Vec<NativeRenderProbeContext> = state.in_flight.values().cloned().collect();
    let probe_id = next_native_probe_id(&mut state);
    let should_probe = !overlapping.is_empty();
    let overlap_callers = if overlapping.is_empty() {
        "none".to_string()
    } else {
        overlapping
            .iter()
            .map(|other| other.caller_kind.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(",")
    };
    let same_snapshot_overlap = overlapping
        .iter()
        .any(|other| other.snapshot_key() == context.snapshot_key());
    let mixed_callers_overlap = overlapping
        .iter()
        .any(|other| other.caller_kind != context.caller_kind);
    state.in_flight.insert(probe_id, context.clone());
    NativeRenderProbeDecision {
        probe_id,
        should_probe,
        overlap_count: overlapping.len(),
        overlap_callers,
        same_snapshot_overlap,
        mixed_callers_overlap,
    }
}

fn end_native_render_probe(probe_id: u64) {
    native_render_probe_state()
        .lock()
        .expect("native render probe state lock should not be poisoned")
        .in_flight
        .remove(&probe_id);
}

fn format_u64_hex(value: u64) -> String {
    format!("0x{value:016x}")
}

fn format_native_probe_common_fields(
    context: &NativeRenderProbeContext,
    decision: &NativeRenderProbeDecision,
) -> String {
    format!(
        "probe_id={} {} overlap_count={} overlap_callers={} same_snapshot_overlap={} mixed_callers_overlap={}",
        decision.probe_id,
        context.format_fields(),
        decision.overlap_count,
        decision.overlap_callers,
        decision.same_snapshot_overlap,
        decision.mixed_callers_overlap,
    )
}

fn format_native_probe_before_line(
    context: &NativeRenderProbeContext,
    decision: &NativeRenderProbeDecision,
    patched_cfg: &CoreConfig,
    event_count: usize,
    total_samples: u64,
) -> String {
    format!(
        "native-probe before {} patch_path={:?} events={} total_samples={}",
        format_native_probe_common_fields(context, decision),
        patched_cfg.patch_path.as_deref().unwrap_or("<none>"),
        event_count,
        total_samples,
    )
}

fn format_requested_native_probe_before_line(
    context: &NativeRenderProbeContext,
    decision: &NativeRenderProbeDecision,
    requested_patch_path: Option<&str>,
) -> String {
    format!(
        "native-probe before {} requested_patch_path={:?}",
        format_native_probe_common_fields(context, decision),
        requested_patch_path.unwrap_or("<none>"),
    )
}

fn format_native_probe_after_line(
    context: &NativeRenderProbeContext,
    decision: &NativeRenderProbeDecision,
) -> String {
    format!(
        "native-probe after {}",
        format_native_probe_common_fields(context, decision),
    )
}

fn emit_native_probe_log(line: String) {
    let logger = native_probe_logger()
        .lock()
        .expect("native probe logger lock should not be poisoned")
        .clone();
    if let Some(logger) = logger {
        logger(&line);
    }
}

fn with_native_render_probe<T, F>(
    probe_context: Option<&NativeRenderProbeContext>,
    patched_cfg: &CoreConfig,
    event_count: usize,
    total_samples: u64,
    render: F,
) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let mut probe_guard = probe_context.map(|context| {
        NativeRenderProbeGuard::enter_prepared(context, patched_cfg, event_count, total_samples)
    });
    let result = render();
    if let Some(probe_guard) = probe_guard.as_mut() {
        probe_guard.mark_returned();
    }
    result
}

fn with_requested_native_render_probe<T, F>(
    probe_context: Option<&NativeRenderProbeContext>,
    requested_patch_path: Option<&str>,
    render: F,
) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let mut probe_guard = probe_context
        .map(|context| NativeRenderProbeGuard::enter_requested(context, requested_patch_path));
    let result = render();
    if let Some(probe_guard) = probe_guard.as_mut() {
        probe_guard.mark_returned();
    }
    result
}

#[cfg(test)]
fn clear_native_render_probe_state_for_tests() {
    let mut state = native_render_probe_state()
        .lock()
        .expect("native render probe state lock should not be poisoned");
    state.in_flight.clear();
    state.next_probe_id = 0;
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
    let (patched_cfg, events, total_samples) = {
        let _guard = CACHE_RENDER_PREPARE_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap();
        prepare_cache_render(mml, cfg)?
    };
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
fn prepare_cache_render(
    mml: &str,
    cfg: &CoreConfig,
) -> Result<(CoreConfig, Vec<midi::TimedMidiEvent>, u64)> {
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

    Ok((patched_cfg, events, total_samples))
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
