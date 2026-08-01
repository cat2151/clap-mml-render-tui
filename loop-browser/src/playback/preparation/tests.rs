use super::*;

fn clip(category: Option<&str>) -> LoopPlaybackClip {
    LoopPlaybackClip {
        path: PathBuf::from("kick.wav"),
        span_measures: 1,
        kind: cmrt_loop_domain::loop_wav_analysis::LoopWavKind::Loop,
        bpm: Some(99.0),
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
    other_bpm.bpm = Some(100.0);
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
    slow.bpm = Some(60.0);
    let mut fast = clip(None);
    fast.path = PathBuf::from("fast.wav");
    fast.bpm = Some(200.0);
    let job = PrepareJob {
        generation: 1,
        reason: LoopGridChange::Initial,
        grid: vec![vec![Some(slow), Some(fast)]],
        submitted_at: Instant::now(),
        background: false,
    };
    let latest_generation = AtomicU64::new(1);
    let mut cache = HashMap::new();
    let diagnostics = crate::playback::diagnostics::new_shared();

    let prepared = prepare_grid(&job, &latest_generation, &mut cache, &diagnostics).unwrap();

    assert_eq!(prepared.target_bpm.bpm, 120.0);
    assert!(!prepared.target_bpm.has_common_range);
    assert!(prepared
        .warning
        .as_deref()
        .is_some_and(|warning| warning.contains("共通BPMなし（BPM120を維持）")));
}

#[test]
fn every_submission_gets_a_new_generation_without_debounce() {
    let mut worker = PreparationWorker::spawn(crate::playback::diagnostics::new_shared());
    let first = worker.submit(Vec::new(), LoopGridChange::Initial);
    let second = worker.submit(Vec::new(), LoopGridChange::Category);
    assert_eq!(second, first + 1);
    assert_eq!(worker.latest_generation.load(Ordering::Acquire), second);
}
