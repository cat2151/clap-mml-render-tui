use super::*;

#[test]
fn draw_shows_effect_chain_overlay_with_instrument_stages_and_add_list() {
    use cmrt_core::{AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset};

    let plugin = AudioEffectPluginInfo::new(
        "Test FX",
        "/clap/does-not-exist.clap",
        "org.example.fx",
        "/presets/does-not-exist",
    );
    let preset = AudioEffectPreset {
        plugin: plugin.key.clone(),
        json_key: plugin.json_key.clone(),
        value: "Hall".to_string(),
        display: "Test FX: Hall".to_string(),
        path: std::path::PathBuf::from("/presets/does-not-exist/Hall"),
    };
    let mut app = build_test_app();
    app.effect_plugins = cmrt_offline_render::EffectPlugins::with_catalog(
        AudioEffectCatalog::with_entries(vec![plugin], vec![preset]),
    );
    app.mode = DawMode::EffectChain;
    app.overlays.effect_chain = crate::overlays::DawEffectChainOverlayState::open(
        2,
        "Pads/Pad 1.fxp".to_string(),
        vec![
            serde_json::json!({"Test FX preset": "Hall"}),
            serde_json::json!({"Unknown preset": "x"}),
        ],
    );

    let chain = render_lines(&app, 120, 30).join("\n");
    assert!(chain.contains("EFFECT CHAIN"), "screen:\n{chain}");
    assert!(
        chain.contains("instrument: Pads/Pad 1.fxp"),
        "screen:\n{chain}"
    );
    assert!(chain.contains("▶ 1. Test FX: Hall"), "screen:\n{chain}");
    assert!(
        chain.contains(r#"2. {"Unknown preset":"x"}"#),
        "screen:\n{chain}"
    );
    // 全角文字は TestBackend で後ろに空白が入るので、空白を落として比べる。
    assert!(
        chain.replace(' ', "").contains("dd:削除"),
        "screen:\n{chain}"
    );
    assert!(!app.uses_textarea_cursor());

    app.mode = DawMode::EffectChainAdd;
    let add = render_lines(&app, 120, 30).join("\n");
    assert!(add.contains("add preset"), "screen:\n{add}");
    assert!(add.contains("▶ Test FX: Hall"), "screen:\n{add}");
    assert!(
        add.replace(' ', "").contains("Enter:末尾へ追加"),
        "screen:\n{add}"
    );
}
