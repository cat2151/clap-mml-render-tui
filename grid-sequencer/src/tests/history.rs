use super::*;

#[test]
fn shift_h_opens_and_closes_the_grid_history() {
    let patches = one_patch();
    let mut screen = silent_screen();

    screen.handle_key(
        shift_press(KeyCode::Char('H')),
        Instant::now(),
        &ready_ctx(&patches),
    );
    assert!(screen.history_open());

    screen.handle_key(
        shift_press(KeyCode::Char('H')),
        Instant::now(),
        &ready_ctx(&patches),
    );
    assert!(!screen.history_open());
}

#[test]
fn lowercase_h_does_not_open_the_grid_history() {
    let patches = one_patch();
    let mut screen = silent_screen();

    screen.handle_key(
        press(KeyCode::Char('h')),
        Instant::now(),
        &ready_ctx(&patches),
    );

    assert!(!screen.history_open());
}

fn screen_with_one_history(now: Instant) -> GridSequencerScreen {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.start(now);
    screen.state.poll_steps(now, std::time::Duration::ZERO);
    screen.absorb_history_snapshots();
    screen
}

#[test]
fn opening_history_automatically_requests_the_first_measure_preview() {
    let now = Instant::now();
    let mut screen = screen_with_one_history(now);
    let patches = Vec::new();
    let ctx = ready_ctx(&patches);

    let open = screen.handle_key(shift_press(KeyCode::Char('H')), now, &ctx);
    assert!(matches!(open, GridSequencerAction::PlayDailyDawPreview(_)));
    assert!(screen.history_previewing());
    assert!(!screen.is_playing(), "SHIFT+H must stop Grid playback");

    screen.set_history_preview_status(GridHistoryPreviewStatus::Playing);
    let stop = screen.handle_key(press(KeyCode::Char(' ')), now, &ctx);
    assert!(matches!(stop, GridSequencerAction::StopDailyDawPreview));
    assert!(!screen.history_previewing());

    let replay = screen.handle_key(press(KeyCode::Char(' ')), now, &ctx);
    assert!(matches!(
        replay,
        GridSequencerAction::PlayDailyDawPreview(_)
    ));
    assert!(!screen.is_playing(), "Grid playback must yield to preview");
}

#[test]
fn finished_preview_replays_from_cache_and_active_preview_can_be_stopped() {
    let now = Instant::now();
    let mut screen = screen_with_one_history(now);
    let patches = Vec::new();
    let ctx = ready_ctx(&patches);
    screen.handle_key(shift_press(KeyCode::Char('H')), now, &ctx);

    screen.set_history_preview_status(GridHistoryPreviewStatus::Finished);
    assert!(matches!(
        screen.handle_key(press(KeyCode::Char(' ')), now, &ctx),
        GridSequencerAction::PlayDailyDawPreview(_)
    ));

    screen.set_history_preview_status(GridHistoryPreviewStatus::Playing);
    assert!(matches!(
        screen.handle_key(press(KeyCode::Char(' ')), now, &ctx),
        GridSequencerAction::StopDailyDawPreview
    ));
    assert!(
        !screen.is_playing(),
        "stopping preview must keep Grid stopped while History is open"
    );

    screen.handle_key(press(KeyCode::Esc), now, &ctx);
    assert!(screen.is_playing(), "closing History must resume the Grid");
}

#[test]
fn closing_history_restores_only_a_previously_playing_grid() {
    let now = Instant::now();
    let patches = Vec::new();
    let ctx = ready_ctx(&patches);
    let mut playing = screen_with_one_history(now);
    let later = now + step_offset(2);
    playing.state.poll_steps(later, std::time::Duration::ZERO);
    assert_ne!(playing.state.step_index(), 0);
    playing.handle_key(shift_press(KeyCode::Char('H')), later, &ctx);
    assert!(!playing.is_playing());
    assert!(matches!(
        playing.handle_key(press(KeyCode::Esc), later, &ctx),
        GridSequencerAction::StopDailyDawPreview
    ));
    assert!(playing.is_playing());
    assert_eq!(playing.state.step_index(), 0);
    assert!(!playing.history_open());

    let mut stopped = screen_with_one_history(now);
    stopped.stop_playing();
    stopped.handle_key(shift_press(KeyCode::Char('H')), now, &ctx);
    stopped.handle_key(press(KeyCode::Char(' ')), now, &ctx);
    stopped.handle_key(press(KeyCode::Esc), now, &ctx);
    assert!(!stopped.is_playing());
}

#[test]
fn closing_history_and_resuming_does_not_duplicate_the_current_grid() {
    let now = Instant::now();
    let patches = Vec::new();
    let ctx = ready_ctx(&patches);
    let mut screen = screen_with_one_history(now);

    screen.handle_key(shift_press(KeyCode::Char('H')), now, &ctx);
    screen.handle_key(shift_press(KeyCode::Char('H')), now, &ctx);
    screen.state.poll_steps(now, std::time::Duration::ZERO);
    screen.absorb_history_snapshots();

    assert_eq!(screen.history_rows().len(), 1);
}

#[test]
fn importing_history_does_not_resume_the_grid() {
    let now = Instant::now();
    let patches = Vec::new();
    let ctx = ready_ctx(&patches);
    let mut screen = screen_with_one_history(now);

    screen.handle_key(shift_press(KeyCode::Char('H')), now, &ctx);
    assert!(matches!(
        screen.handle_key(press(KeyCode::Enter), now, &ctx),
        GridSequencerAction::ImportToDailyDaw(_)
    ));

    assert!(!screen.history_open());
    assert!(!screen.is_playing());
}
