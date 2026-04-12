use super::*;
use std::{
    path::Path,
    sync::{Arc, Barrier, Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn reexports_core_config() {
    let config = CoreConfig {
        output_midi: "out.mid".into(),
        output_wav: "out.wav".into(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patch_path: Some("/patches/Pad 1.fxp".into()),
        patches_dir: Some("/patches".into()),
        random_patch: false,
    };

    assert_eq!(config.output_midi, "out.mid");
    assert_eq!(config.output_wav, "out.wav");
    assert_eq!(config.sample_rate, 44_100.0);
    assert_eq!(config.buffer_size, 512);
    assert_eq!(config.patch_path.as_deref(), Some("/patches/Pad 1.fxp"));
    assert_eq!(config.patches_dir.as_deref(), Some("/patches"));
    assert!(!config.random_patch);
}

#[test]
fn reexports_patch_helpers() {
    assert_eq!(
        to_relative("/patches", Path::new("/patches/Pads/Pad 1.fxp")),
        "Pads/Pad 1.fxp"
    );
}

#[test]
fn cache_render_extracts_patch_from_embedded_json() {
    let patches_dir = std::path::PathBuf::from("patches");
    let config = CoreConfig {
        output_midi: "out.mid".into(),
        output_wav: "out.wav".into(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patch_path: Some("/patches/Default.fxp".into()),
        patches_dir: Some(patches_dir.to_string_lossy().into_owned()),
        random_patch: true,
    };

    let patch = extract_patch_from_json(Some(r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#), &config);

    let expected = patches_dir.join("Pads").join("Pad 1.fxp");
    assert_eq!(patch.as_deref(), Some(expected.to_string_lossy().as_ref()));
}

#[test]
fn cache_render_returns_none_when_json_patch_is_missing() {
    let config = CoreConfig {
        output_midi: "out.mid".into(),
        output_wav: "out.wav".into(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patch_path: Some("/patches/Default.fxp".into()),
        patches_dir: Some("/patches".into()),
        random_patch: true,
    };

    let patch = extract_patch_from_json(Some(r#"{"tempo":120}"#), &config);

    assert_eq!(patch, None);
}

#[test]
fn cache_render_prepares_memory_only_render_inputs() {
    let patches_dir = std::path::PathBuf::from("patches");
    let config = CoreConfig {
        output_midi: "out.mid".into(),
        output_wav: "out.wav".into(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patch_path: Some("patches/Default.fxp".into()),
        patches_dir: Some(patches_dir.to_string_lossy().into_owned()),
        random_patch: true,
    };

    let (patched_cfg, events, total_samples) =
        prepare_cache_render(r#"{"Surge XT patch":"Pads/Pad 1.fxp"}t120o4c"#, &config)
            .expect("cache render inputs should be prepared");

    assert_eq!(
        patched_cfg.patch_path.as_deref(),
        Some(
            patches_dir
                .join("Pads")
                .join("Pad 1.fxp")
                .to_string_lossy()
                .as_ref()
        )
    );
    assert!(
        !patched_cfg.random_patch,
        "random patch selection should be disabled for cache renders"
    );
    assert!(!events.is_empty(), "valid MML should produce MIDI events");
    assert!(
        total_samples > 0,
        "valid MML should produce a positive sample length"
    );
}

#[test]
fn cache_render_extracts_patch_from_embedded_json_with_factory_prefix_fallback() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("cmrt_core_patch_fallback_{suffix}"));
    let factory_patch = root.join("patches_factory").join("Pads").join("Pad 1.fxp");
    std::fs::create_dir_all(factory_patch.parent().unwrap()).unwrap();
    std::fs::write(&factory_patch, b"dummy").unwrap();

    let config = CoreConfig {
        output_midi: "out.mid".into(),
        output_wav: "out.wav".into(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patch_path: Some("/patches/Default.fxp".into()),
        patches_dir: Some(root.to_string_lossy().into_owned()),
        random_patch: false,
    };

    let patch = extract_patch_from_json(Some(r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#), &config);

    assert_eq!(
        patch.as_deref(),
        Some(factory_patch.to_string_lossy().as_ref())
    );
    std::fs::remove_dir_all(root).ok();
}

fn native_probe_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct NativeProbeTestGuard;

impl Drop for NativeProbeTestGuard {
    fn drop(&mut self) {
        set_native_probe_logger(None);
        clear_native_render_probe_state_for_tests();
    }
}

fn with_captured_native_probe_logs<F>(test: F)
where
    F: FnOnce(Arc<Mutex<Vec<String>>>),
{
    let _lock = native_probe_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    clear_native_render_probe_state_for_tests();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let logger: NativeProbeLogger = {
        let captured = Arc::clone(&captured);
        Arc::new(move |line: &str| {
            captured.lock().unwrap().push(line.to_string());
        })
    };
    set_native_probe_logger(Some(logger));
    let _guard = NativeProbeTestGuard;
    test(captured);
}

fn probe_test_core_config() -> CoreConfig {
    CoreConfig {
        output_midi: "out.mid".into(),
        output_wav: "out.wav".into(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patch_path: Some("/patches/Pad 1.fxp".into()),
        patches_dir: Some("/patches".into()),
        random_patch: false,
    }
}

#[test]
fn native_render_probe_skips_logs_when_no_overlap_is_detected() {
    with_captured_native_probe_logs(|captured| {
        let config = probe_test_core_config();
        let context = NativeRenderProbeContext::cache_worker(1, 2, 3, 0x1234, 4);

        with_native_render_probe(Some(&context), &config, 8, 1_024, || Ok(()))
            .expect("probe wrapper should return inner result");

        assert!(
            captured.lock().unwrap().is_empty(),
            "probe logs should stay silent without overlapping native renders"
        );
    });
}

#[test]
fn native_render_probe_emits_before_and_after_for_overlapping_render() {
    with_captured_native_probe_logs(|captured| {
        let config = probe_test_core_config();
        let entered_render = Arc::new(Barrier::new(2));
        let release_render = Arc::new(Barrier::new(2));
        let first_context = NativeRenderProbeContext::cache_worker(1, 2, 7, 0xaaaa, 4);
        let first_config = config.clone();
        let entered_render_worker = Arc::clone(&entered_render);
        let release_render_worker = Arc::clone(&release_render);

        let handle = std::thread::spawn(move || {
            with_native_render_probe(Some(&first_context), &first_config, 12, 2_048, || {
                entered_render_worker.wait();
                release_render_worker.wait();
                Ok(())
            })
            .expect("first render should complete");
        });

        entered_render.wait();

        let second_context = NativeRenderProbeContext::playback_current(2, 0, 2, 0xbbbb, 4);
        with_native_render_probe(Some(&second_context), &config, 6, 512, || Ok(()))
            .expect("second render should complete");

        release_render.wait();
        handle.join().unwrap();

        let logs = captured.lock().unwrap().clone();
        assert_eq!(logs.len(), 2, "expected a paired before/after probe log");
        assert!(
            logs[0].contains("native-probe before"),
            "before log missing: {:?}",
            logs
        );
        assert!(
            logs[1].contains("native-probe after"),
            "after log missing: {:?}",
            logs
        );
        assert!(
            logs.iter()
                .all(|line| line.contains("caller=playback_current")),
            "paired logs should describe the overlapping render: {:?}",
            logs
        );
        assert!(
            logs.iter().all(|line| line.contains("overlap_count=1")),
            "paired logs should record the detected overlap count: {:?}",
            logs
        );
        assert!(
            logs.iter()
                .all(|line| line.contains("overlap_callers=cache_worker")),
            "paired logs should record the in-flight caller kind: {:?}",
            logs
        );
    });
}

#[test]
fn requested_native_render_probe_emits_paired_logs_for_tui_overlap() {
    with_captured_native_probe_logs(|captured| {
        let first_context = NativeRenderProbeContext::tui_prefetch(1, 0x1111, 4);
        let second_context = NativeRenderProbeContext::tui_playback(23, 2, 0x2222, 4);
        let entered_render = Arc::new(Barrier::new(2));
        let release_render = Arc::new(Barrier::new(2));
        let entered_render_worker = Arc::clone(&entered_render);
        let release_render_worker = Arc::clone(&release_render);

        let handle = std::thread::spawn(move || {
            with_requested_native_render_probe(
                Some(&first_context),
                Some("patches_factory/Pads/Pad 1.fxp"),
                || {
                    entered_render_worker.wait();
                    release_render_worker.wait();
                    Ok(())
                },
            )
            .expect("first requested render should complete");
        });

        entered_render.wait();
        with_requested_native_render_probe(
            Some(&second_context),
            Some("patches_factory/Pads/Pad 2.fxp"),
            || Ok(()),
        )
        .expect("second requested render should complete");

        release_render.wait();
        handle.join().unwrap();

        let logs = captured.lock().unwrap().clone();
        assert_eq!(logs.len(), 2, "expected paired TUI probe logs");
        assert!(
            logs.iter().all(|line| line.contains("caller=tui_playback")),
            "TUI logs should describe the overlapping playback render: {:?}",
            logs
        );
        assert!(
            logs.iter()
                .any(|line| line.contains("requested_patch_path")),
            "requested render logs should include the requested patch path: {:?}",
            logs
        );
        assert!(
            logs.iter().all(|line| line.contains("active_renders=2")),
            "TUI logs should retain the active render count: {:?}",
            logs
        );
    });
}
