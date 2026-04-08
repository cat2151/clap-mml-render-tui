use super::*;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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
