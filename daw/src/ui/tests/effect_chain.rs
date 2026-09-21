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
        name: "Hall".to_string(),
        category: "Test Category".to_string(),
        kind: "Reverb 2".to_string(),
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
            serde_json::json!({"Test FX preset": "Hall", "bypass": true}),
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
    assert!(
        chain.contains("3. [bypass] Test FX: Hall"),
        "screen:\n{chain}"
    );
    // 全角文字は TestBackend で後ろに空白が入るので、空白を落として比べる。
    assert!(
        chain.replace(' ', "").contains("dd:削除"),
        "screen:\n{chain}"
    );
    assert!(!app.uses_textarea_cursor());

    app.mode = DawMode::EffectChainAdd;
    app.overlays.effect_chain.add =
        crate::overlays::DawEffectAddState::open(app.effect_plugins.catalog().unwrap());
    let add = render_lines(&app, 120, 30).join("\n");
    assert!(add.contains("add preset"), "screen:\n{add}");
    assert!(add.contains("category"), "screen:\n{add}");
    assert!(add.contains("kind"), "screen:\n{add}");
    assert!(add.contains("list"), "screen:\n{add}");
    assert!(!add.contains("role"), "screen:\n{add}");
    assert!(add.contains("all"), "screen:\n{add}");
    // list の行に plugin 名の列がある(preset.name だけでなく plugin 名も出る)。
    assert!(add.contains("Test FX"), "screen:\n{add}");
    assert!(add.contains("Hall"), "screen:\n{add}");
    assert!(
        add.replace(' ', "").contains("Enter:末尾へ追加"),
        "screen:\n{add}"
    );
    // query 欄が category/kind/list の上に描かれる（placeholder は編集していないときだけ出る）。
    assert!(
        add.replace(' ', "").contains("listを絞り込み"),
        "screen:\n{add}"
    );
    assert!(!app.uses_textarea_cursor());
}

#[test]
fn draw_shows_the_query_editing_hint_and_uses_the_textarea_cursor_while_filtering() {
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
        name: "Hall".to_string(),
        category: "Test Category".to_string(),
        kind: "Reverb 2".to_string(),
        path: std::path::PathBuf::from("/presets/does-not-exist/Hall"),
    };
    let mut app = build_test_app();
    app.effect_plugins = cmrt_offline_render::EffectPlugins::with_catalog(
        AudioEffectCatalog::with_entries(vec![plugin], vec![preset]),
    );
    app.mode = DawMode::EffectChainAdd;
    app.overlays.effect_chain.add =
        crate::overlays::DawEffectAddState::open(app.effect_plugins.catalog().unwrap());
    app.overlays.effect_chain.add.begin_filter();

    let screen = render_lines(&app, 120, 30).join("\n");
    assert!(
        screen.replace(' ', "").contains("Enter=確定/ESC=中断"),
        "screen:\n{screen}"
    );
    assert!(app.uses_textarea_cursor());
}

#[test]
fn add_list_keeps_the_cursor_inside_the_scroll_margin_and_remembers_the_offset() {
    use cmrt_core::{AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset};

    let plugin = AudioEffectPluginInfo::new(
        "Test FX",
        "/clap/does-not-exist.clap",
        "org.example.fx",
        "/presets/does-not-exist",
    );
    let presets: Vec<AudioEffectPreset> = (0..60)
        .map(|n| AudioEffectPreset {
            plugin: plugin.key.clone(),
            json_key: plugin.json_key.clone(),
            value: format!("p{n:02}"),
            display: format!("Test FX: p{n:02}"),
            name: format!("p{n:02}"),
            category: "Test Category".to_string(),
            kind: "Reverb 2".to_string(),
            path: std::path::PathBuf::from(format!("/presets/does-not-exist/p{n:02}")),
        })
        .collect();
    let mut app = build_test_app();
    app.effect_plugins = cmrt_offline_render::EffectPlugins::with_catalog(
        AudioEffectCatalog::with_entries(vec![plugin], presets),
    );
    app.mode = DawMode::EffectChainAdd;
    app.overlays.effect_chain.add =
        crate::overlays::DawEffectAddState::open(app.effect_plugins.catalog().unwrap());

    let visible_range = |app: &DawApp| {
        let screen = render_lines(app, 120, 30).join("\n");
        let visible: Vec<usize> = (0..60)
            .filter(|n| screen.contains(&format!("p{n:02}")))
            .collect();
        (
            *visible.first().expect("list is visible"),
            *visible.last().expect("list is visible"),
        )
    };

    // 下へ大きく動かすと、カーソルの下に余白ぶんの行が残る位置まで scroll する。
    app.overlays.effect_chain.add.list_cursor = 40;
    let (first, last) = visible_range(&app);
    assert!(first <= 40 && 40 <= last, "visible {first}..={last}");
    let rows = last - first + 1;
    let margin = rows * 30 / 100;
    assert!(margin >= 1, "rows={rows}");
    assert_eq!(last - 40, margin, "visible {first}..={last}");

    // 余白の内側へ戻るだけの移動では表示先頭を動かさない。
    app.overlays.effect_chain.add.list_cursor = 38;
    assert_eq!(visible_range(&app), (first, last));

    // 上の余白を越えると、カーソルの上に余白ぶんの行が残る位置まで scroll する。
    app.overlays.effect_chain.add.list_cursor = 20;
    let (first, _) = visible_range(&app);
    assert_eq!(20 - first, margin);
}
