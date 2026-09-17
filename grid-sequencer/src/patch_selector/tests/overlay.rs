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

    let selector = selector(&screen);
    assert_eq!(selector.instance, 1);
    assert_eq!(selector.selected_patch(), Some("Keys/Beta.fxp"));
}

/// `t` は j/k で選んだ行を開く。click と同じ行が開くので、その後の操作は共通。
#[test]
fn t_opens_the_selected_tracks_patch_selector() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.rows_mut()[1].patch = Some("Keys/Beta.fxp".to_string());
    screen.select_track(1);

    screen.handle_key(press(KeyCode::Char('t')), Instant::now(), &ctx);

    let selector = selector(&screen);
    assert_eq!(selector.instance, 1);
    assert_eq!(selector.selected_patch(), Some("Keys/Beta.fxp"));
}

/// 開けないときは黙って戻らず、必ず理由を通知に残す。理由ごとに次の一手が違う。
#[test]
fn loading_error_empty_and_unconfigured_catalogs_report_why_they_do_not_open() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    let loading = ctx_with(
        crate::tests::loading_patch_load(),
        crate::tests::empty_catalog(),
        &Voicing,
    );
    screen.open_patch_selector(0, &loading);
    assert!(screen.patch_selector.is_none());
    assert_eq!(notice_reason(&screen), PatchUnavailable::Loading);

    let failed = PatchLoadState::Err("catalog failed".to_string());
    let error = ctx_with(&failed, crate::tests::empty_catalog(), &Voicing);
    screen.open_patch_selector(0, &error);
    assert!(screen.patch_selector.is_none());
    assert_eq!(
        notice_reason(&screen),
        PatchUnavailable::LoadError("catalog failed".to_string())
    );

    let empty = context(crate::tests::empty_patch_load());
    screen.open_patch_selector(0, &empty);
    assert!(screen.patch_selector.is_none());
    assert_eq!(notice_reason(&screen), PatchUnavailable::NoPatches);

    let patches = patches();
    let mut unconfigured = context(&patches);
    unconfigured.patch_dirs_configured = false;
    screen.open_patch_selector(0, &unconfigured);
    assert!(screen.patch_selector.is_none());
    assert_eq!(notice_reason(&screen), PatchUnavailable::NotConfigured);
    assert!(screen.cycle_random().patch);
}

/// 「一覧はあるが和音行の poly 絞りで消えた」は、一覧 0 件とは別の理由として出す。
#[test]
fn a_chord_row_without_poly_patches_reports_the_filter_as_the_reason() {
    let patches = PatchLoadState::ready(vec![(
        "Bass/Mono.fxp".to_string(),
        "bass/mono.fxp".to_string(),
    )]);
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    screen.open_patch_selector(CHORD_ROW, &ctx);

    assert!(screen.patch_selector.is_none());
    assert_eq!(notice_reason(&screen), PatchUnavailable::NoPolyPatches);
}

/// 開けたときに前の通知が残っていると、直った理由が画面に居座る。
#[test]
fn opening_the_selector_clears_a_previous_notice() {
    let patches = patches();
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    let mut unconfigured = context(&patches);
    unconfigured.patch_dirs_configured = false;
    screen.open_patch_selector(0, &unconfigured);
    assert!(screen.patch_notice_open());

    screen.open_patch_selector(0, &context(&patches));

    assert!(screen.patch_selector.is_some());
    assert!(!screen.patch_notice_open());
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

    let selector = selector(&screen);
    assert_eq!(selector.total(), 3);
    assert_eq!(
        filtered_patches(selector),
        ["Keys/Alpha.fxp", "Keys/Beta.fxp", "Pads/Poly.fxp"]
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

    assert_eq!(selector(&screen).instance, 2);
    // arpeggio 行は Lead で開く。Role を ALL に戻せば mono の音色も選べる。
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Home), &ctx);
    let mono = filtered_patches(selector(&screen))
        .iter()
        .position(|patch| *patch == "Bass/Mono.fxp")
        .unwrap();
    screen.patch_selector.as_mut().unwrap().patch_cursor = mono;
    assert_eq!(selector(&screen).selected_patch(), Some("Bass/Mono.fxp"));
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
    let disappeared = PatchLoadState::Err("catalog disappeared".to_string());
    let error = ctx_with(&disappeared, crate::tests::empty_catalog(), &Voicing);

    screen.handle_patch_selector_key(press(KeyCode::Enter), &error);

    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );
    assert!(screen.cycle_random().patch);
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
    assert!(!screen.cycle_random().patch);

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
fn escape_keeps_patch_random_off_when_it_was_already_off() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.set_cycle_random(crate::CycleRandomItem::Patch, false);

    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Esc), &ctx);

    assert!(screen.patch_selector.is_none());
    assert!(!screen.cycle_random().patch);
}

#[test]
fn confirming_the_current_patch_holds_patch_random_and_undo_restores_it() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());

    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);

    assert!(screen.patch_selector.is_none());
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );
    assert!(!screen.cycle_random().patch);

    screen.handle_key(press(KeyCode::Char('u')), Instant::now(), &ctx);
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

/// 設定不足でカタログから外れたプラグインの案内は、selector が開いている間ずっと
/// 枠の下辺に出る。一覧 0 件の通知（[`PatchUnavailable`]）とは別の軸で、
/// **一覧が十分にあるときにも出る**のがこの案内の要点。
#[test]
fn the_selector_carries_the_reason_a_plugin_is_missing_from_the_catalog() {
    let patches = patches();
    let mut ctx = context(&patches);
    let notes = vec!["Vaporizer2 は patches_dirs が無いため一覧に出ません".to_string()];
    ctx.catalog_notes = &notes;
    let mut screen = GridSequencerScreen::with_track_count(None, 2);

    screen.open_patch_selector(0, &ctx);

    let selector = selector(&screen);
    assert_eq!(selector.catalog_notes(), notes.as_slice());
    // 一覧そのものは開けている（0 件の通知とは別の軸であることの担保）。
    assert!(selector.total() > 0);
}

/// 外れたプラグインが無ければ 1 文字も出さない。
#[test]
fn the_selector_shows_no_note_when_every_plugin_is_in_the_catalog() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 2);

    screen.open_patch_selector(0, &ctx);

    assert!(screen
        .patch_selector
        .as_ref()
        .unwrap()
        .catalog_notes()
        .is_empty());
}
