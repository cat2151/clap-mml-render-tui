use super::*;
use crate::loop_wav_analysis::{LoopAnalysisSource, LoopTempoAnalysis, LoopWavKind};
use crate::loop_waveform::{LoopWaveform, WAVEFORM_BINS_PER_MEASURE};

fn create_wav(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for _ in 0..400 {
        writer.write_sample(0_i16).unwrap();
    }
    writer.finalize().unwrap();
}

fn indexed(relative: &str) -> LoopWavIndex {
    LoopWavIndex {
        relative: relative.to_string(),
        analysis: LoopWavAnalysis {
            duration_seconds: 4.0,
            kind: LoopWavKind::Loop,
            tempo: Some(LoopTempoAnalysis {
                bpm: 120.0,
                declared_bpm: None,
                beats: 8,
                meter_numerator: 4,
                meter_denominator: 4,
                source: LoopAnalysisSource::DurationEstimate,
            }),
            measures: 2,
        },
        waveform: LoopWaveform::silent(2),
    }
}

#[test]
fn build_index_collects_only_wav_files_and_sorts_them() {
    let temp = tempfile_dir("collect");
    create_wav(&temp.join("Pack/Bass/z.WAV"));
    create_wav(&temp.join("Pack/Bass/A.wav"));
    std::fs::write(temp.join("Pack/Bass/readme.txt"), b"test").unwrap();
    let mut events = Vec::new();

    let (index, skipped) = build_index(&[temp.to_string_lossy().into_owned()], &mut |event| {
        events.push(event)
    })
    .unwrap();

    assert_eq!(skipped, 0);
    assert_eq!(index.roots.len(), 1);
    assert_eq!(
        index.roots[0]
            .wav_files
            .iter()
            .map(|wav| wav.relative.as_str())
            .collect::<Vec<_>>(),
        [
            PathBuf::from("Pack")
                .join("Bass")
                .join("A.wav")
                .to_string_lossy()
                .into_owned(),
            PathBuf::from("Pack")
                .join("Bass")
                .join("z.WAV")
                .to_string_lossy()
                .into_owned(),
        ]
    );
    assert_eq!(events[0], LoopScanProgress::Started { roots: 1 });
    let analyzing = events
        .iter()
        .filter(|event| matches!(event, LoopScanProgress::Analyzing { .. }))
        .collect::<Vec<_>>();
    assert!(matches!(
        analyzing[0],
        LoopScanProgress::Analyzing { current: 1, total: 2, path }
            if path.ends_with("A.wav")
    ));
    assert!(matches!(
        analyzing[1],
        LoopScanProgress::Analyzing { current: 2, total: 2, path }
            if path.ends_with("z.WAV")
    ));
}

#[test]
fn one_shots_path_component_forces_one_shot_without_acid_metadata() {
    let temp = tempfile_dir("one-shots");
    create_wav(&temp.join("Pack/ONE SHOTS/Kick.wav"));

    let (index, skipped) =
        build_index(&[temp.to_string_lossy().into_owned()], &mut |_| {}).unwrap();
    let wav = &index.roots[0].wav_files[0];

    assert_eq!(skipped, 0);
    assert_eq!(wav.analysis.kind, LoopWavKind::OneShot);
    assert_eq!(wav.analysis.tempo, None);
    assert_eq!(wav.analysis.measures, 1);
    assert_eq!(wav.waveform.rms_db_tenths.len(), WAVEFORM_BINS_PER_MEASURE);
}

#[test]
fn one_shots_path_matching_requires_a_complete_component() {
    assert!(is_one_shot_relative_path("Pack/One Shots/Kick.wav"));
    assert!(is_one_shot_relative_path("one shots/Kick.wav"));
    assert!(!is_one_shot_relative_path("Pack/One Shot Samples/Kick.wav"));
    assert!(!is_one_shot_relative_path("Pack/My One Shots/Kick.wav"));
}

#[test]
fn scan_skips_invalid_wav_and_persists_successful_analysis() {
    let temp = tempfile_dir("skip-invalid");
    let _guard = crate::test_utils::set_local_dir_envs(&temp);
    let root = temp.join("loops");
    create_wav(&root.join("Good.wav"));
    std::fs::write(root.join("Broken.wav"), b"not a wave").unwrap();
    let dirs = vec![root.to_string_lossy().into_owned()];
    let mut events = Vec::new();

    let summary = scan_dirs_and_save_with_progress(&dirs, |event| events.push(event)).unwrap();

    assert_eq!(
        summary,
        LoopScanSummary {
            roots: 1,
            wav_files: 1,
            skipped_wav_files: 1,
        }
    );
    assert!(events.iter().any(|event| matches!(
        event,
        LoopScanProgress::Skipped { path, error }
            if path.ends_with("Broken.wav") && error.contains("RIFF header")
    )));
    let bytes = std::fs::read(loop_index_path().unwrap()).unwrap();
    let index: LoopIndex = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(index.roots[0].wav_files.len(), 1);
    assert_eq!(index.roots[0].wav_files[0].relative, "Good.wav");
    assert_eq!(index.roots[0].wav_files[0].analysis.duration_seconds, 4.0);
    assert_eq!(
        index.roots[0].wav_files[0].analysis.tempo.unwrap().bpm,
        120.0
    );
    assert_eq!(index.roots[0].wav_files[0].waveform.rms_db_tenths.len(), 64);
}

#[test]
fn scan_saves_an_empty_index_when_every_wav_is_invalid() {
    let temp = tempfile_dir("all-invalid");
    let _guard = crate::test_utils::set_local_dir_envs(&temp);
    let root = temp.join("loops");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("Broken.wav"), b"not a wave").unwrap();

    let summary = scan_dirs_and_save(&[root.to_string_lossy().into_owned()]).unwrap();

    assert_eq!(summary.wav_files, 0);
    assert_eq!(summary.skipped_wav_files, 1);
    let bytes = std::fs::read(loop_index_path().unwrap()).unwrap();
    let index: LoopIndex = serde_json::from_slice(&bytes).unwrap();
    assert!(index.roots[0].wav_files.is_empty());
}

#[test]
fn scan_replaces_cache_and_root_failure_preserves_previous_cache() {
    let temp = tempfile_dir("save");
    let _guard = crate::test_utils::set_local_dir_envs(&temp);
    let first_root = temp.join("first");
    create_wav(&first_root.join("One.wav"));
    let first_dirs = vec![first_root.to_string_lossy().into_owned()];
    let first_summary = scan_dirs_and_save(&first_dirs).unwrap();
    assert_eq!(first_summary.wav_files, 1);
    let cache_path = loop_index_path().unwrap();
    let first_cache = std::fs::read(&cache_path).unwrap();

    create_wav(&first_root.join("Two.wav"));
    let second_summary = scan_dirs_and_save(&first_dirs).unwrap();
    assert_eq!(second_summary.wav_files, 2);
    let second_cache = std::fs::read(&cache_path).unwrap();
    assert_ne!(first_cache, second_cache);

    let missing_dirs = vec![temp.join("missing").to_string_lossy().into_owned()];
    assert!(scan_dirs_and_save(&missing_dirs).is_err());
    assert_eq!(std::fs::read(cache_path).unwrap(), second_cache);
}

#[test]
fn validate_index_rejects_version_roots_and_parent_paths() {
    let valid = LoopIndex {
        version: LOOP_INDEX_VERSION,
        roots: vec![LoopRootIndex {
            path: "/loops".to_string(),
            wav_files: vec![indexed("pack/kick.wav")],
        }],
    };
    validate_index(&valid, &["/loops".to_string()]).unwrap();

    let mut wrong_version = valid.clone();
    wrong_version.version += 1;
    assert!(validate_index(&wrong_version, &["/loops".to_string()]).is_err());
    let mut old_version = valid.clone();
    old_version.version = 3;
    assert!(validate_index(&old_version, &["/loops".to_string()]).is_err());
    assert!(validate_index(&valid, &["/other".to_string()]).is_err());

    let mut unsafe_path = valid.clone();
    unsafe_path.roots[0].wav_files = vec![indexed("../outside.wav")];
    assert!(validate_index(&unsafe_path, &["/loops".to_string()]).is_err());

    let mut invalid_waveform = valid;
    invalid_waveform.roots[0].wav_files[0]
        .waveform
        .rms_db_tenths
        .pop();
    assert!(validate_index(&invalid_waveform, &["/loops".to_string()]).is_err());
}

#[test]
fn legacy_index_deserializes_far_enough_to_report_the_version_mismatch() {
    let legacy = serde_json::json!({
        "version": LOOP_INDEX_VERSION - 1,
        "roots": [{
            "path": "/loops",
            "wav_files": [{
                "relative": "kick.wav",
                "analysis": {
                    "duration_seconds": 2.0,
                    "bpm": 120.0,
                    "beats": 4,
                    "meter_numerator": 4,
                    "meter_denominator": 4,
                    "measures": 1,
                    "source": "duration_estimate"
                },
                "waveform": LoopWaveform::silent(1)
            }]
        }]
    });
    let index: LoopIndex = serde_json::from_value(legacy).unwrap();
    let error = validate_index(&index, &["/loops".to_string()]).unwrap_err();

    assert!(error.to_string().contains("versionが一致しません"));
}

fn tempfile_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "cmrt_loop_library_{label}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}
