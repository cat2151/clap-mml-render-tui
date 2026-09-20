use super::*;
use mmlabc_to_smf::mml_preprocessor;
use std::{
    path::Path,
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
    };

    let patch = extract_patch_from_json(Some(r#"{"tempo":120}"#), &config);

    assert_eq!(patch, None);
}

#[test]
fn cache_render_extracts_patch_from_embedded_json_with_factory_prefix_fallback() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "cmrt_core_patch_fallback_{}_{suffix}",
        std::process::id()
    ));
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
        ..Default::default()
    };

    let patch = extract_patch_from_json(Some(r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#), &config);

    assert_eq!(
        patch.as_deref(),
        Some(factory_patch.to_string_lossy().as_ref())
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn mml_with_resolved_embedded_patch_keeps_core_patch_value_relative_to_base() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "cmrt_core_patch_rewrite_{}_{suffix}",
        std::process::id()
    ));
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
        ..Default::default()
    };

    let rewritten =
        mml_with_resolved_embedded_patch(r#"{"Surge XT patch":"Pads/Pad 1.fxp"}t120o4c"#, &config);
    let preprocessed = mml_preprocessor::extract_embedded_json(rewritten.as_ref());
    let value: serde_json::Value =
        serde_json::from_str(preprocessed.embedded_json.as_deref().unwrap()).unwrap();
    let expected_patch = std::path::Path::new("patches_factory")
        .join("Pads")
        .join("Pad 1.fxp")
        .to_string_lossy()
        .into_owned();

    assert_eq!(
        value.get("Surge XT patch").and_then(|patch| patch.as_str()),
        Some(expected_patch.as_str())
    );
    assert_eq!(preprocessed.remaining_mml, "t120o4c");
    std::fs::remove_dir_all(root).ok();
}

/// 音色の書き換えは `"Surge XT patch"` だけを触り、effect chain などの他のキーは残す。
#[test]
fn mml_with_resolved_embedded_patch_keeps_the_effect_chain() {
    let config = CoreConfig {
        output_midi: "out.mid".into(),
        output_wav: "out.wav".into(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patch_path: None,
        patches_dir: Some("/patches".into()),
        random_patch: false,
        ..Default::default()
    };
    let chain = serde_json::json!([
        { "TONE3000 preset": "Bogner Fullstack" },
        { "Surge XT Effects preset": "Reverb 1/Cathedral 2.srgfx" }
    ]);
    let mml = format!(
        r#"{{"Surge XT patch":"Pads/Pad 1.fxp","{EFFECT_CHAIN_JSON_KEY}":{chain}}}t120o4c"#
    );

    let rewritten = mml_with_resolved_embedded_patch(&mml, &config);
    let preprocessed = mml_preprocessor::extract_embedded_json(rewritten.as_ref());
    let value: serde_json::Value =
        serde_json::from_str(preprocessed.embedded_json.as_deref().unwrap()).unwrap();

    assert_eq!(value.get(EFFECT_CHAIN_JSON_KEY), Some(&chain));
    assert!(value.get("Surge XT patch").is_some());
    assert_eq!(preprocessed.remaining_mml, "t120o4c");
}

/// どのプラグインで鳴らすかの判別は、base で解決する**前**の display 文字列で行う。
/// base 自体がプラグインごとに違うので、解決を先にすると鶏と卵になる。
///
/// 実装は play server 側（`cmrt_core::embedded_patch_ref`）の再輸出。あちらに
/// テストが無いので、この契約はここで押さえておく。
#[test]
fn embedded_patch_ref_returns_the_unresolved_display_string() {
    assert_eq!(
        embedded_patch_ref(r#"{"Surge XT patch":"Dexed_01.syx/00 Say Again."}cde"#).as_deref(),
        Some("Dexed_01.syx/00 Say Again.")
    );
    assert_eq!(
        embedded_patch_ref(r#"{"Surge XT patch":"Pads/Pad 1.fxp"}t120o4c"#).as_deref(),
        Some("Pads/Pad 1.fxp")
    );
}

/// 先頭 JSON が無い / 音色キーが無い MML は `None`。呼び出し側はこれを
/// 「既定プラグイン」として扱う（`docs/adr/0004-default-plugin-owns-unspecified-patches.md`）。
#[test]
fn embedded_patch_ref_is_none_without_a_patch_key() {
    assert_eq!(embedded_patch_ref("cde"), None);
    assert_eq!(embedded_patch_ref(r#"{"tempo":120}cde"#), None);
}
