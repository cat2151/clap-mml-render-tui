use std::collections::HashMap;
use std::fs::Metadata;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Instant;
use std::time::UNIX_EPOCH;

use rubberband_ffi::StretchProfile;

use crate::loop_time_stretch::{
    format_bpm, prepare_path, profile_for_category, PreparedAudio, TargetBpm,
};
use crate::tui::loop_browser::{LoopGridChange, LoopPlaybackClip, LoopPlaybackGrid};

use super::diagnostics::{
    duration_seconds, log_event, path_label, SharedLoopStretchDiagnostics, StretchStatus,
};
use super::tempo::grid_target_bpm;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct AudioKey {
    path: PathBuf,
    bpm_bits: Option<u64>,
    target_bpm_bits: u64,
    profile: StretchProfile,
}

impl AudioKey {
    fn new(clip: &LoopPlaybackClip, target_bpm: f64) -> Self {
        Self {
            path: clip.path.clone(),
            bpm_bits: clip.source_bpm().map(f64::to_bits),
            target_bpm_bits: target_bpm.to_bits(),
            profile: profile_for_category(clip.category.as_deref()),
        }
    }
}

pub(super) type PreparedEntry = Result<Arc<PreparedAudio>, Arc<str>>;

pub(super) struct PreparedSet {
    pub(super) generation: u64,
    pub(super) grid: LoopPlaybackGrid,
    pub(super) target_bpm: TargetBpm,
    audio: HashMap<AudioKey, PreparedEntry>,
    pub(super) warning: Option<String>,
}

impl PreparedSet {
    pub(super) fn audio_for(&self, clip: &LoopPlaybackClip) -> Option<&PreparedEntry> {
        self.audio.get(&AudioKey::new(clip, self.target_bpm.bpm))
    }
}

struct PrepareJob {
    generation: u64,
    reason: LoopGridChange,
    grid: LoopPlaybackGrid,
    submitted_at: Instant,
}

pub(super) struct PreparationResult {
    pub(super) generation: u64,
    pub(super) prepared: PreparedSet,
}

enum PreparationCommand {
    Prepare(PrepareJob),
    Stop,
}

pub(super) struct PreparationWorker {
    sender: mpsc::Sender<PreparationCommand>,
    receiver: mpsc::Receiver<PreparationResult>,
    latest_generation: Arc<AtomicU64>,
    diagnostics: SharedLoopStretchDiagnostics,
    next_generation: u64,
    worker: Option<JoinHandle<()>>,
}

impl PreparationWorker {
    pub(super) fn spawn(diagnostics: SharedLoopStretchDiagnostics) -> Self {
        let (command_sender, command_receiver) = mpsc::channel();
        let (result_sender, result_receiver) = mpsc::channel();
        let latest_generation = Arc::new(AtomicU64::new(0));
        let worker_generation = Arc::clone(&latest_generation);
        let worker_diagnostics = Arc::clone(&diagnostics);
        let worker = std::thread::spawn(move || {
            preparation_loop(
                command_receiver,
                result_sender,
                &worker_generation,
                &worker_diagnostics,
            )
        });
        Self {
            sender: command_sender,
            receiver: result_receiver,
            latest_generation,
            diagnostics,
            next_generation: 0,
            worker: Some(worker),
        }
    }

    pub(super) fn submit(&mut self, grid: LoopPlaybackGrid, reason: LoopGridChange) -> u64 {
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        let generation = self.next_generation;
        self.latest_generation.store(generation, Ordering::Release);
        let target_bpm = grid_target_bpm(&grid);
        self.diagnostics_begin(generation, reason, &grid, target_bpm);
        let _ = self.sender.send(PreparationCommand::Prepare(PrepareJob {
            generation,
            reason,
            grid,
            submitted_at: Instant::now(),
        }));
        generation
    }

    fn diagnostics_begin(
        &self,
        generation: u64,
        reason: LoopGridChange,
        grid: &LoopPlaybackGrid,
        target_bpm: TargetBpm,
    ) {
        self.diagnostics
            .lock()
            .unwrap()
            .begin(generation, reason, grid, target_bpm);
        log_event(format!(
            "event=preparation-submitted generation={generation} reason={} tracks={} measures={} clip_cells={} target_bpm={} common_range={}",
            reason.label(),
            grid.len(),
            grid.iter().map(Vec::len).max().unwrap_or(0),
            grid.iter().flatten().filter(|clip| clip.is_some()).count(),
            format_bpm(target_bpm.bpm),
            target_bpm.has_common_range,
        ));
    }

    pub(super) fn try_result(&self) -> Option<PreparationResult> {
        self.receiver.try_recv().ok()
    }

    pub(super) fn cancel(&self) {
        self.latest_generation.fetch_add(1, Ordering::AcqRel);
    }
}

impl Drop for PreparationWorker {
    fn drop(&mut self) {
        self.cancel();
        let _ = self.sender.send(PreparationCommand::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn preparation_loop(
    receiver: mpsc::Receiver<PreparationCommand>,
    result_sender: mpsc::Sender<PreparationResult>,
    latest_generation: &AtomicU64,
    diagnostics: &SharedLoopStretchDiagnostics,
) {
    let mut cache = HashMap::<CacheKey, Arc<PreparedAudio>>::new();
    while let Ok(command) = receiver.recv() {
        match command {
            PreparationCommand::Prepare(job) => {
                if latest_generation.load(Ordering::Acquire) != job.generation {
                    log_event(format!(
                        "event=preparation-discarded generation={} reason={} outcome=stale-before-start",
                        job.generation,
                        job.reason.label(),
                    ));
                    continue;
                }
                log_event(format!(
                    "event=preparation-worker-start generation={} reason={} queue_ms={}",
                    job.generation,
                    job.reason.label(),
                    job.submitted_at.elapsed().as_millis(),
                ));
                if let Some(prepared) =
                    prepare_grid(&job, latest_generation, &mut cache, diagnostics)
                {
                    let _ = result_sender.send(PreparationResult {
                        generation: job.generation,
                        prepared,
                    });
                }
            }
            PreparationCommand::Stop => return,
        }
    }
}

fn prepare_grid(
    job: &PrepareJob,
    latest_generation: &AtomicU64,
    cache: &mut HashMap<CacheKey, Arc<PreparedAudio>>,
    diagnostics: &SharedLoopStretchDiagnostics,
) -> Option<PreparedSet> {
    let started_at = Instant::now();
    let target_bpm = grid_target_bpm(&job.grid);
    let mut audio = HashMap::<AudioKey, PreparedEntry>::new();
    let mut errors = Vec::new();
    let mut prepared_count = 0usize;
    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;
    log_event(format!(
        "event=preparation-start generation={} reason={} target_bpm={} common_range={} rubberband_revision={}",
        job.generation,
        job.reason.label(),
        format_bpm(target_bpm.bpm),
        target_bpm.has_common_range,
        super::diagnostics::rubberband_revision(),
    ));
    for clip in job.grid.iter().flatten().filter_map(Option::as_ref) {
        if latest_generation.load(Ordering::Acquire) != job.generation {
            log_cancelled(job, started_at, prepared_count, cache_hits, cache_misses);
            return None;
        }
        let audio_key = AudioKey::new(clip, target_bpm.bpm);
        if audio.contains_key(&audio_key) {
            continue;
        }
        let metadata = std::fs::metadata(&clip.path).ok();
        let cache_key = CacheKey::new(&audio_key, metadata.as_ref());
        let (prepared, cache_hit) = if let Some(cached) = cache.get(&cache_key) {
            cache_hits += 1;
            (Ok(Arc::clone(cached)), true)
        } else {
            cache_misses += 1;
            let result = prepare_path(
                &clip.path,
                clip.source_bpm(),
                target_bpm.bpm,
                clip.category.as_deref(),
                || latest_generation.load(Ordering::Acquire) != job.generation,
            )
            .map(Arc::new)
            .map_err(|error| Arc::<str>::from(error.to_string()));
            if let Ok(value) = &result {
                cache.insert(cache_key, Arc::clone(value));
            }
            (result, false)
        };
        if latest_generation.load(Ordering::Acquire) != job.generation {
            log_cancelled(job, started_at, prepared_count, cache_hits, cache_misses);
            return None;
        }
        if let Err(error) = &prepared {
            errors.push(format!("{}: {error}", clip.path.display()));
            diagnostics.lock().unwrap().set_status(
                job.generation,
                clip,
                target_bpm.bpm,
                StretchStatus::Error(error.to_string()),
            );
            log_event(format!(
                "event=clip-prepared generation={} outcome=error path=\"{}\" source_bpm={} target_bpm={} ratio={:.6} category={} profile={} cache={} error=\"{}\"",
                job.generation,
                path_label(&clip.path),
                source_bpm_label(clip),
                format_bpm(target_bpm.bpm),
                requested_time_ratio(clip, target_bpm.bpm),
                clip.category.as_deref().unwrap_or("none"),
                profile_label(clip),
                if cache_hit { "hit" } else { "miss" },
                error.replace('"', "\\\""),
            ));
        } else if let Ok(value) = &prepared {
            prepared_count += 1;
            let info = value.info();
            diagnostics.lock().unwrap().set_status(
                job.generation,
                clip,
                target_bpm.bpm,
                StretchStatus::Ready { info, cache_hit },
            );
            log_event(format!(
                "event=clip-prepared generation={} outcome={} path=\"{}\" source_bpm={} target_bpm={} ratio={:.6} category={} profile={} cache={} channels={} sample_rate={} input_frames={} rubberband_frames={} output_frames={} input_seconds={:.6} output_seconds={:.6}",
                job.generation,
                if clip.is_one_shot() { "one-shot-bypass" } else if (info.time_ratio - 1.0).abs() < f64::EPSILON { "bypass" } else { "stretched" },
                path_label(&clip.path),
                source_bpm_label(clip),
                format_bpm(target_bpm.bpm),
                info.time_ratio,
                clip.category.as_deref().unwrap_or("none"),
                super::diagnostics::profile_label(info.profile),
                if cache_hit { "hit" } else { "miss" },
                info.channels,
                info.sample_rate,
                info.input_frames,
                info.rubberband_output_frames,
                info.output_frames,
                duration_seconds(info.input_frames, info.sample_rate),
                duration_seconds(info.output_frames, info.sample_rate),
            ));
        }
        audio.insert(audio_key, prepared);
    }
    if latest_generation.load(Ordering::Acquire) != job.generation {
        log_cancelled(job, started_at, prepared_count, cache_hits, cache_misses);
        return None;
    }
    let warning = (!target_bpm.has_common_range || !errors.is_empty()).then(|| {
        let omitted = errors.len().saturating_sub(2);
        let mut details = Vec::new();
        if !target_bpm.has_common_range {
            details.push("配置clipに共通BPMなし（BPM120を維持）".to_string());
        }
        details.extend(errors.iter().take(2).cloned());
        let mut message = format!("BPM{}: {}", format_bpm(target_bpm.bpm), details.join(" / "));
        if omitted > 0 {
            message.push_str(&format!(" / 他{omitted}件"));
        }
        if !errors.is_empty() {
            message.push_str("（対象clipは無音、他clipは再生継続）");
        }
        message
    });
    log_event(format!(
        "event=preparation-finished generation={} outcome={} total_ms={} prepared={} errors={} cache_hits={} cache_misses={} target_bpm={}",
        job.generation,
        if errors.is_empty() { "ok" } else { "partial-error" },
        started_at.elapsed().as_millis(),
        prepared_count,
        errors.len(),
        cache_hits,
        cache_misses,
        format_bpm(target_bpm.bpm),
    ));
    Some(PreparedSet {
        generation: job.generation,
        grid: job.grid.clone(),
        target_bpm,
        audio,
        warning,
    })
}

fn log_cancelled(
    job: &PrepareJob,
    started_at: Instant,
    prepared: usize,
    cache_hits: usize,
    cache_misses: usize,
) {
    log_event(format!(
        "event=preparation-cancelled generation={} reason={} total_ms={} prepared={} cache_hits={} cache_misses={}",
        job.generation,
        job.reason.label(),
        started_at.elapsed().as_millis(),
        prepared,
        cache_hits,
        cache_misses,
    ));
}

#[derive(Eq, Hash, PartialEq)]
struct CacheKey {
    audio: AudioKey,
    file_len: u64,
    modified_nanos: Option<u128>,
}

impl CacheKey {
    fn new(audio: &AudioKey, metadata: Option<&Metadata>) -> Self {
        Self {
            audio: audio.clone(),
            file_len: metadata.map_or(0, Metadata::len),
            modified_nanos: metadata
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos()),
        }
    }
}

pub(super) fn profile_label(clip: &LoopPlaybackClip) -> &'static str {
    match profile_for_category(clip.category.as_deref()) {
        StretchProfile::Drum => "drum/R2",
        StretchProfile::General => "general/R3",
    }
}

fn source_bpm_label(clip: &LoopPlaybackClip) -> String {
    if clip.is_one_shot() {
        return "one-shot".to_string();
    }
    clip.bpm
        .map(format_bpm)
        .unwrap_or_else(|| "one-shot".to_string())
}

fn requested_time_ratio(clip: &LoopPlaybackClip, target_bpm: f64) -> f64 {
    if clip.is_one_shot() {
        1.0
    } else {
        clip.bpm.map_or(1.0, |bpm| bpm / target_bpm)
    }
}

#[cfg(test)]
#[path = "preparation/tests.rs"]
mod tests;
