use super::*;

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
    assert!(!screen.cycle_random().patch);
    assert!(!screen.state.has_pending_cycle());
    let layout = PatchSelectorLayout::new(AREA, false);
    let beta = filtered_patches(selector(&screen))
        .iter()
        .position(|patch| *patch == "Keys/Beta.fxp")
        .unwrap();

    screen.handle_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            layout.patch_rows.x,
            layout.patch_rows.y + beta as u16,
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
fn clicking_the_table_header_does_nothing_and_the_row_below_it_applies_the_first_patch() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Beta.fxp".to_string());
    screen.open_patch_selector(0, &ctx);
    let layout = PatchSelectorLayout::new(AREA, false);
    let first = filtered_patches(selector(&screen))[0].to_string();
    assert_ne!(first, "Keys/Beta.fxp");

    screen.handle_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            layout.patch_rows.x,
            layout.patch_rows.y - 1,
        ),
        AREA,
        &ctx,
    );
    assert!(
        screen.patch_selector.is_some(),
        "header の click は何もしない"
    );
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Beta.fxp")
    );

    screen.handle_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            layout.patch_rows.x,
            layout.patch_rows.y,
        ),
        AREA,
        &ctx,
    );
    assert!(screen.patch_selector.is_none());
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some(first.as_str())
    );
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
            layout.patch_rows.x,
            layout.patch_rows.y,
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
