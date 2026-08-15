use super::*;

fn searchable_patches() -> Vec<(String, String)> {
    [
        "Guitars/Soft Strum.fxp",
        "Guitars/Warm Strum Pad.fxp",
        "Pads/Soft Cloud.fxp",
        "Strum/Plain Pad.fxp",
    ]
    .into_iter()
    .map(|patch| (patch.to_string(), patch.to_lowercase()))
    .collect()
}

fn type_text(screen: &mut GridSequencerScreen, ctx: &GridSequencerContext<'_>, text: &str) {
    for ch in text.chars() {
        screen.handle_patch_selector_key(press(KeyCode::Char(ch)), ctx);
    }
}

#[test]
fn slash_searches_filename_stems_case_insensitively_with_and_terms() {
    let patches = searchable_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "STRUM warm");

    let selector = screen.patch_selector.as_ref().unwrap();
    assert!(selector.name_search_active);
    assert_eq!(selector.name_query(), "STRUM warm");
    assert_eq!(
        selector
            .categories
            .iter()
            .map(|category| (category.name.as_str(), category.patches.len()))
            .collect::<Vec<_>>(),
        vec![("全カテゴリ", 1), ("Guitars", 1)]
    );
    assert_eq!(
        selector.selected_patch(),
        Some("Guitars/Warm Strum Pad.fxp")
    );
}

#[test]
fn directory_and_extension_text_do_not_match_the_patch_name() {
    let patches = searchable_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "strum");

    let selector = screen.patch_selector.as_ref().unwrap();
    assert_eq!(selector.categories[0].patches.len(), 2);
    assert!(!selector.categories[0]
        .patches
        .iter()
        .any(|patch| patch == "Strum/Plain Pad.fxp"));

    for _ in 0..5 {
        screen.handle_patch_selector_key(press(KeyCode::Backspace), &ctx);
    }
    type_text(&mut screen, &ctx, "fxp");
    let selector = screen.patch_selector.as_ref().unwrap();
    assert_eq!(selector.categories.len(), 1);
    assert!(selector.categories[0].patches.is_empty());
}

#[test]
fn an_empty_query_keeps_the_original_categories_without_the_pseudo_category() {
    let patches = searchable_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);

    let selector = screen.patch_selector.as_ref().unwrap();
    assert!(selector.name_search_active);
    assert!(!selector.has_name_query());
    assert_eq!(
        selector
            .categories
            .iter()
            .map(|category| category.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Guitars", "Pads", "Strum"]
    );
}

#[test]
fn typing_does_not_preview_and_enter_confirms_the_search_with_one_preview() {
    let patches = searchable_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Pads/Soft Cloud.fxp".to_string());
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "warm");
    assert_eq!(
        screen
            .patch_selector
            .as_ref()
            .unwrap()
            .previewed_patch
            .as_deref(),
        Some("Pads/Soft Cloud.fxp")
    );

    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);

    let selector = screen.patch_selector.as_ref().unwrap();
    assert!(!selector.name_search_active);
    assert_eq!(
        selector.selected_patch(),
        Some("Guitars/Warm Strum Pad.fxp")
    );
    assert_eq!(
        selector.previewed_patch.as_deref(),
        Some("Guitars/Warm Strum Pad.fxp")
    );
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Pads/Soft Cloud.fxp"),
        "search confirmation previews but does not commit"
    );

    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);
    assert!(screen.patch_selector.is_none());
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Guitars/Warm Strum Pad.fxp")
    );
}

#[test]
fn escape_restores_the_query_and_selection_from_before_input() {
    let patches = searchable_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Pads/Soft Cloud.fxp".to_string());
    screen.open_patch_selector(0, &ctx);
    assert_eq!(
        screen.patch_selector.as_ref().unwrap().selected_patch(),
        Some("Pads/Soft Cloud.fxp")
    );

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "strum");
    screen.handle_patch_selector_key(press(KeyCode::Esc), &ctx);

    let selector = screen.patch_selector.as_ref().unwrap();
    assert!(!selector.name_search_active);
    assert_eq!(selector.name_query(), "");
    assert_eq!(selector.selected_category().name, "Pads");
    assert_eq!(selector.selected_patch(), Some("Pads/Soft Cloud.fxp"));
}

#[test]
fn applying_with_no_hits_keeps_the_selector_open() {
    let patches = searchable_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "no-such-patch");
    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::End), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Down), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('r')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);

    let selector = screen.patch_selector.as_ref().unwrap();
    assert_eq!(selector.categories.len(), 1);
    assert!(selector.categories[0].patches.is_empty());
    assert_eq!(selector.selected_patch(), None);
}

#[test]
fn random_draws_only_from_name_search_hits_and_returns_to_all_categories() {
    let patches = searchable_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    type_text(&mut screen, &ctx, "strum");
    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);
    screen.patch_selector.as_mut().unwrap().category_cursor = 1;
    screen.patch_selector.as_mut().unwrap().patch_cursor = 0;

    screen.handle_patch_selector_key(press(KeyCode::Char('r')), &ctx);

    let selector = screen.patch_selector.as_ref().unwrap();
    assert_eq!(selector.category_cursor, 0);
    assert!(selector
        .selected_patch()
        .is_some_and(|patch| patch.contains("Strum")));
    assert_ne!(selector.selected_patch(), Some("Strum/Plain Pad.fxp"));
}

#[test]
fn clicking_the_name_search_field_does_not_close_the_selector() {
    let patches = searchable_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('/')), &ctx);
    let name_search = PatchSelectorLayout::new(AREA, true).name_search.unwrap();

    screen.handle_patch_selector_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            name_search.x,
            name_search.y,
        ),
        AREA,
        &ctx,
    );

    assert!(screen.patch_selector.is_some());
}
