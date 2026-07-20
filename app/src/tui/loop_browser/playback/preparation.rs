use std::collections::HashMap;
use std::fs::Metadata;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::UNIX_EPOCH;

use rubberband_ffi::StretchProfile;

use crate::loop_time_stretch::{
    format_bpm, prepare_path, profile_for_category, PreparedAudio, TargetBpm,
};
use crate::tui::loop_browser::{LoopPlaybackClip, LoopPlaybackGrid};

use super::tempo::grid_target_bpm;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct AudioKey {
    path: PathBuf,
    bpm_bits: u64,
    target_bpm_bits: u64,
    profile: StretchProfile,
}

impl AudioKey {
    fn new(clip: &LoopPlaybackClip, target_bpm: f64) -> Self {
        Self {
            path: clip.path.clone(),
            bpm_bits: clip.bpm.to_bits(),
            target_bpm_bits: target_bpm.to_bits(),
            profile: profile_for_category(clip.category.as_deref()),
        }
    }
}

pub(super) type PreparedEntry = Result<Arc<PreparedAudio>, Arc<str>>;

pub(super) struct PreparedSet {
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
    grid: LoopPlaybackGrid,
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
    next_generation: u64,
    worker: Option<JoinHandle<()>>,
}

impl PreparationWorker {
    pub(super) fn spawn() -> Self {
        let (command_sender, command_receiver) = mpsc::channel();
        let (result_sender, result_receiver) = mpsc::channel();
        let latest_generation = Arc::new(AtomicU64::new(0));
        let worker_generation = Arc::clone(&latest_generation);
        let worker = std::thread::spawn(move || {
            preparation_loop(command_receiver, result_sender, &worker_generation)
        });
        Self {
            sender: command_sender,
            receiver: result_receiver,
            latest_generation,
            next_generation: 0,
            worker: Some(worker),
        }
    }

    pub(super) fn submit(&mut self, grid: LoopPlaybackGrid) -> u64 {
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        let generation = self.next_generation;
        self.latest_generation.store(generation, Ordering::Release);
        let _ = self
            .sender
            .send(PreparationCommand::Prepare(PrepareJob { generation, grid }));
        generation
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
) {
    let mut cache = HashMap::<CacheKey, Arc<PreparedAudio>>::new();
    while let Ok(command) = receiver.recv() {
        match command {
            PreparationCommand::Prepare(job) => {
                if let Some(prepared) = prepare_grid(&job, latest_generation, &mut cache) {
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
) -> Option<PreparedSet> {
    let target_bpm = grid_target_bpm(&job.grid);
    let mut audio = HashMap::<AudioKey, PreparedEntry>::new();
    let mut errors = Vec::new();
    for clip in job.grid.iter().flatten().filter_map(Option::as_ref) {
        if latest_generation.load(Ordering::Acquire) != job.generation {
            return None;
        }
        let audio_key = AudioKey::new(clip, target_bpm.bpm);
        if audio.contains_key(&audio_key) {
            continue;
        }
        let metadata = std::fs::metadata(&clip.path).ok();
        let cache_key = CacheKey::new(&audio_key, metadata.as_ref());
        let prepared = if let Some(cached) = cache.get(&cache_key) {
            Ok(Arc::clone(cached))
        } else {
            let result = prepare_path(
                &clip.path,
                clip.bpm,
                target_bpm.bpm,
                clip.category.as_deref(),
                || latest_generation.load(Ordering::Acquire) != job.generation,
            )
            .map(Arc::new)
            .map_err(|error| Arc::<str>::from(error.to_string()));
            if let Ok(value) = &result {
                cache.insert(cache_key, Arc::clone(value));
            }
            result
        };
        if let Err(error) = &prepared {
            errors.push(format!("{}: {error}", clip.path.display()));
        }
        audio.insert(audio_key, prepared);
    }
    if latest_generation.load(Ordering::Acquire) != job.generation {
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
    Some(PreparedSet {
        grid: job.grid.clone(),
        target_bpm,
        audio,
        warning,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(category: Option<&str>) -> LoopPlaybackClip {
        LoopPlaybackClip {
            path: PathBuf::from("kick.wav"),
            span_measures: 1,
            bpm: 99.0,
            category: category.map(str::to_string),
            meter_numerator: 4,
            meter_denominator: 4,
        }
    }

    #[test]
    fn audio_key_includes_profile_and_bpm() {
        assert_ne!(
            AudioKey::new(&clip(Some("drum")), 120.0),
            AudioKey::new(&clip(None), 120.0)
        );
        let mut other_bpm = clip(Some("drum"));
        other_bpm.bpm = 100.0;
        assert_ne!(
            AudioKey::new(&clip(Some("drum")), 120.0),
            AudioKey::new(&other_bpm, 120.0)
        );
        assert_ne!(
            AudioKey::new(&clip(Some("drum")), 120.0),
            AudioKey::new(&clip(Some("drum")), 118.75)
        );
    }

    #[test]
    fn profile_label_exposes_selected_algorithm() {
        assert_eq!(profile_label(&clip(Some("drum"))), "drum/R2");
        assert_eq!(profile_label(&clip(Some("bass"))), "general/R3");
    }

    #[test]
    fn incompatible_grid_warns_that_bpm_120_is_kept() {
        let mut slow = clip(None);
        slow.path = PathBuf::from("slow.wav");
        slow.bpm = 60.0;
        let mut fast = clip(None);
        fast.path = PathBuf::from("fast.wav");
        fast.bpm = 200.0;
        let job = PrepareJob {
            generation: 1,
            grid: vec![vec![Some(slow), Some(fast)]],
        };
        let latest_generation = AtomicU64::new(1);
        let mut cache = HashMap::new();

        let prepared = prepare_grid(&job, &latest_generation, &mut cache).unwrap();

        assert_eq!(prepared.target_bpm.bpm, 120.0);
        assert!(!prepared.target_bpm.has_common_range);
        assert!(prepared
            .warning
            .as_deref()
            .is_some_and(|warning| warning.contains("共通BPMなし（BPM120を維持）")));
    }

    #[test]
    fn every_submission_gets_a_new_generation_without_debounce() {
        let mut worker = PreparationWorker::spawn();
        let first = worker.submit(Vec::new());
        let second = worker.submit(Vec::new());
        assert_eq!(second, first + 1);
        assert_eq!(worker.latest_generation.load(Ordering::Acquire), second);
    }
}
