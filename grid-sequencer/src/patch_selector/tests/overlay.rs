use super::*;

#[test]
fn ready_catalog_opens_at_the_rows_current_patch() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.rows_mut()[1].patch = Some("Keys/Beta.fxp".to_string());

    screen.handle_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            patch_column(&screen),
            3,
        ),
        AREA,
        &ctx,
    );

    let selector = screen.patch_selector.as_ref().unwrap();
    assert_eq!(selector.instance, 1);
    assert_eq!(selector.selected_category().name, "Keys");
    assert_eq!(selector.selected_patch(), Some("Keys/Beta.fxp"));
}

#[test]
fn loading_error_empty_and_unconfigured_catalogs_do_not_open() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    let loading = ctx_with(
        GridPatchLoad::Loading,
        crate::tests::empty_catalog(),
        &Voicing,
    );
    screen.open_patch_selector(0, &loading);
    assert!(screen.patch_selector.is_none());

    let error = ctx_with(
        GridPatchLoad::Err("catalog failed"),
        crate::tests::empty_catalog(),
        &Voicing,
    );
    screen.open_patch_selector(0, &error);
    assert!(screen.patch_selector.is_none());

    let empty = context(&[]);
    screen.open_patch_selector(0, &empty);
    assert!(screen.patch_selector.is_none());

    let patches = patches();
    let mut unconfigured = context(&patches);
    unconfigured.patch_dirs_configured = false;
    screen.open_patch_selector(0, &unconfigured);
    assert!(screen.patch_selector.is_none());
}

#[test]
fn clicking_a_patch_changes_only_that_row_enters_hold_and_cancels_pending_cycle() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.state.rows_mut()[1].patch = Some("Bass/Mono.fxp".to_string());
    screen.state.stage_next_cycle(
        vec![GridRow::default(); 2],
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap(),
    );
    screen.open_patch_selector(0, &ctx);
    let layout = PatchSelectorLayout::new(AREA, false);
    let beta = screen
        .patch_selector
        .as_ref()
        .unwrap()
        .selected_category()
        .patches
        .iter()
        .position(|patch| patch == "Keys/Beta.fxp")
        .unwrap();

    screen.handle_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            layout.patch_list.x,
            layout.patch_list.y + beta as u16,
        ),
        AREA,
        &ctx,
    );

    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Beta.fxp")
    );
    assert_eq!(
        screen.state.rows()[1].patch.as_deref(),
        Some("Bass/Mono.fxp")
    );
    assert!(!screen.cycle_random().patch);
    assert!(!screen.state.has_pending_cycle());
    assert!(screen.patch_selector.is_none());

    screen.handle_key(press(KeyCode::Char('u')), Instant::now(), &ctx);
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );
    assert!(screen.cycle_random().patch);
}

#[test]
fn chord_row_selector_contains_only_confirmed_poly_patches() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    screen.open_patch_selector(CHORD_ROW, &ctx);

    let selector = screen.patch_selector.as_ref().unwrap();
    let candidates = selector
        .categories
        .iter()
        .flat_map(|category| category.patches.iter())
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        candidates,
        vec!["Keys/Alpha.fxp", "Keys/Beta.fxp", "Pads/Poly.fxp"]
    );
}

#[test]
fn child_lane_opens_its_shared_instance_selector_and_allows_a_mono_patch() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    // 上から2番目のvoice row（行6）から、instance共有のPATCH欄をclickする。
    screen.handle_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            patch_column(&screen),
            6,
        ),
        AREA,
        &ctx,
    );

    let selector = screen.patch_selector.as_ref().unwrap();
    assert_eq!(selector.instance, 2);
    assert!(selector
        .categories
        .iter()
        .flat_map(|category| &category.patches)
        .any(|patch| patch == "Bass/Mono.fxp"));
    screen.patch_selector.as_mut().unwrap().category_cursor = 0;
    screen.patch_selector.as_mut().unwrap().patch_cursor = 0;
    assert_eq!(
        screen.patch_selector.as_ref().unwrap().selected_patch(),
        Some("Bass/Mono.fxp")
    );
    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);

    assert_eq!(
        screen.state.instances()[2].patch.as_deref(),
        Some("Bass/Mono.fxp")
    );
    assert_eq!(screen.state.instances()[2].lanes.len(), 4);
}

#[test]
fn selector_revalidates_the_catalog_before_applying() {
    let patches = patches();
    let ready = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ready);
    screen.patch_selector.as_mut().unwrap().patch_cursor = 1;
    let error = ctx_with(
        GridPatchLoad::Err("catalog disappeared"),
        crate::tests::empty_catalog(),
        &Voicing,
    );

    screen.handle_patch_selector_key(press(KeyCode::Enter), &error);

    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );
    assert!(screen.cycle_random().patch);
}

#[test]
fn mouse_wheel_previews_the_cursor_patch_without_committing_until_enter() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ctx);
    let layout = PatchSelectorLayout::new(AREA, false);

    screen.handle_patch_selector_mouse(
        mouse(
            MouseEventKind::ScrollDown,
            layout.patch_list.x,
            layout.patch_list.y,
        ),
        AREA,
        &ctx,
    );
    assert_eq!(
        screen.patch_selector.as_ref().unwrap().selected_patch(),
        Some("Keys/Beta.fxp")
    );
    assert_eq!(
        screen
            .patch_selector
            .as_ref()
            .unwrap()
            .previewed_patch
            .as_deref(),
        Some("Keys/Beta.fxp")
    );
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );

    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Beta.fxp")
    );
}

#[test]
fn r_selects_and_previews_a_random_patch_without_closing_the_overlay() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('r')), &ctx);

    let selector = screen.patch_selector.as_ref().unwrap();
    let selected = selector.selected_patch().unwrap().to_string();
    assert_ne!(selected, "Keys/Alpha.fxp");
    assert_eq!(selector.previewed_patch.as_deref(), Some(selected.as_str()));
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );

    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);
    assert!(screen.patch_selector.is_none());
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some(selected.as_str())
    );
    assert!(!screen.cycle_random().patch);
}

#[test]
fn escape_cancels_a_preview_without_committing_it() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Down), &ctx);
    assert_eq!(
        screen
            .patch_selector
            .as_ref()
            .unwrap()
            .previewed_patch
            .as_deref(),
        Some("Keys/Beta.fxp")
    );
    screen.handle_patch_selector_key(press(KeyCode::Esc), &ctx);

    assert!(screen.patch_selector.is_none());
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );
    assert!(screen.cycle_random().patch);
}

#[test]
fn an_overlay_that_appears_after_opening_cancels_the_selector_without_applying() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ctx);
    screen.patch_selector.as_mut().unwrap().patch_cursor = 1;
    screen.restart_notice = Some(Instant::now());

    screen.handle_key(press(KeyCode::Enter), Instant::now(), &ctx);

    assert!(screen.patch_selector.is_none());
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );
}
