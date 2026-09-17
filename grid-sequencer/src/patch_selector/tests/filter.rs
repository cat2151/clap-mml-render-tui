use super::*;

use std::{collections::BTreeMap, sync::Arc};

use cmrt_tui_core::patch_load::PatchCatalogSnapshot;

fn type_text(screen: &mut GridSequencerScreen, ctx: &GridSequencerContext<'_>, text: &str) {
    for ch in text.chars() {
        screen.handle_patch_selector_key(press(KeyCode::Char(ch)), ctx);
    }
}

/// Vaporizer2 の `PD` は Category `Pad`。表示名に `pad` を含まない音色が Category だけで当たる。
fn categorized_patches() -> PatchLoadState {
    let plugin = cmrt_tui_core::patch_plugins::CatalogPlugin {
        name: "Vaporizer2".to_string(),
        plugin_path: "C:/Vaporizer2.clap".to_string(),
        plugin_id: Some(cmrt_runtime::VAPORIZER2_PLUGIN_ID.to_string()),
        base: Some("C:/presets".to_string()),
        dirs: Vec::new(),
        resolved_patches: None,
        source_notices: Vec::new(),
    };
    let info = cmrt_core::AudioPluginInfo::new(
        plugin.name.clone(),
        plugin.plugin_path.clone(),
        plugin.plugin_id.clone(),
        plugin.base.clone(),
    );
    let audio = ["PD Wide Sky.vvp", "LD Bright Lead.vvp", "PD Soft Pad.vvp"]
        .map(|display| info.describe_patch(display, None));
    assert_eq!(audio[0].selector_category.as_deref(), Some("Pad"));
    let pairs = audio
        .iter()
        .map(|patch| {
            (
                patch.reference.display.clone(),
                patch.normalized_display.clone(),
            )
        })
        .collect();
    PatchLoadState::Ready(Arc::new(PatchCatalogSnapshot::new(
        pairs,
        audio.to_vec(),
        vec![plugin],
        Vec::new(),
        BTreeMap::new(),
    )))
}

#[test]
fn slash_pad_enter_keeps_only_pads_including_a_category_only_hit() {
    let patches = categorized_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    assert!(!selector(&screen).filter_visible());

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    assert!(selector(&screen).filter_editing());
    assert!(selector(&screen).filter_visible());
    type_text(&mut screen, &ctx, "pad");
    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);

    let selector = selector(&screen);
    assert!(!selector.filter_editing());
    assert!(selector.filter_visible());
    assert_eq!(
        filtered_patches(selector),
        ["PD Soft Pad.vvp", "PD Wide Sky.vvp"]
    );
    assert_eq!(selector.filtered_len(), 2);
    assert_eq!(selector.total(), 3);
    assert_eq!(selector.patch_cursor, 0);
    assert_eq!(selector.previewed_patch.as_deref(), Some("PD Soft Pad.vvp"));
}

#[test]
fn terms_are_case_insensitive_regular_expressions_joined_with_and() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "DRUMS kick|snare");

    assert_eq!(
        filtered_patches(selector(&screen)),
        ["Drums/Kick 01.wav", "Drums/Snare 01.wav"]
    );
}

#[test]
fn escape_while_editing_reverts_to_the_committed_condition() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Leads/Lead 01.fxp".to_string());
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "lead");
    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);
    assert_eq!(filtered_patches(selector(&screen)), ["Leads/Lead 01.fxp"]);

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "x");
    assert_eq!(filtered_patches(selector(&screen)), [] as [&str; 0]);
    screen.handle_patch_selector_key(press(KeyCode::Esc), &ctx);

    let selector = selector(&screen);
    assert!(!selector.filter_editing());
    assert!(selector.filter_visible());
    assert_eq!(filtered_patches(selector), ["Leads/Lead 01.fxp"]);
    assert_eq!(selector.selected_patch(), Some("Leads/Lead 01.fxp"));
    assert_eq!(
        selector.previewed_patch.as_deref(),
        Some("Leads/Lead 01.fxp")
    );
    assert!(screen.patch_selector.is_some());
}

#[test]
fn an_invalid_regex_empties_the_list_and_enter_cannot_confirm_a_patch() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Leads/Lead 01.fxp".to_string());
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "(");
    assert!(selector(&screen).filter_error().is_some());
    assert_eq!(selector(&screen).filtered_len(), 0);
    assert_eq!(selector(&screen).selected_patch(), None);

    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);
    assert!(!selector(&screen).filter_editing());
    screen.handle_patch_selector_key(press(KeyCode::Char('r')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);

    assert!(screen.patch_selector.is_some(), "選択が無いので確定しない");
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Leads/Lead 01.fxp")
    );
}

#[test]
fn the_regex_is_anded_with_the_selected_preset() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    // Role=ALL の Preset 一覧にある `Bass › bass|bs`。
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    let bass = selector(&screen)
        .presets()
        .iter()
        .position(|preset| preset.label == "Bass › bass|bs")
        .expect("Role=ALL は他 Role の Preset を qualify 表記で含む");
    for _ in 0..bass {
        screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    }
    assert_eq!(
        filtered_patches(selector(&screen)),
        ["Basses/Bass 01.fxp", "Basses/Bass 02.fxp"]
    );

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "01");

    assert_eq!(filtered_patches(selector(&screen)), ["Basses/Bass 01.fxp"]);
}

#[test]
fn r_draws_only_from_the_filtered_list() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "drums");
    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);

    for _ in 0..20 {
        screen.handle_patch_selector_key(press(KeyCode::Char('r')), &ctx);
        let selected = selector(&screen).selected_patch().unwrap();
        assert!(selected.starts_with("Drums/"), "{selected}");
    }
}

#[test]
fn clicking_the_regex_field_does_not_close_the_selector() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    let query = PatchSelectorLayout::new(AREA, true).query.unwrap();

    screen.handle_patch_selector_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), query.x, query.y),
        AREA,
        &ctx,
    );

    assert!(screen.patch_selector.is_some());
    assert!(selector(&screen).filter_editing());
}
