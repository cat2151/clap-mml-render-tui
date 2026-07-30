use crossterm::event::KeyModifiers;

use super::*;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// SHIFT 付きの押下。crossterm は Shift+r を `Char('R')` + SHIFT で届ける。
fn shift_press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

fn one_patch() -> Vec<(String, String)> {
    vec![("Keys/Piano.fxp".to_string(), "keys/piano.fxp".to_string())]
}

fn ready_ctx(patches: &[(String, String)]) -> GridSequencerContext<'_> {
    GridSequencerContext {
        patch_dirs_configured: true,
        patch_load: GridPatchLoad::Ready(patches),
    }
}

/// MIDI を送らないテスト用の画面。
fn silent_screen() -> GridSequencerScreen {
    GridSequencerScreen::new(None)
}

#[test]
fn q_quits_the_screen() {
    let patches = one_patch();
    let mut screen = silent_screen();

    assert!(matches!(
        screen.handle_key(
            press(KeyCode::Char('q')),
            Instant::now(),
            &ready_ctx(&patches)
        ),
        GridSequencerAction::Quit
    ));
}

#[test]
fn t_cycles_track_count_and_requests_restart() {
    let patches = one_patch();
    let mut screen = GridSequencerScreen::with_track_count(None, 1);

    for expected in [2, 4, 8, 16, 1] {
        let action = screen.handle_key(
            press(KeyCode::Char('t')),
            Instant::now(),
            &ready_ctx(&patches),
        );
        assert!(matches!(
            action,
            GridSequencerAction::RestartWithTrackCount(count) if count == expected
        ));
        assert_eq!(screen.track_count(), expected);
        assert_eq!(screen.state.rows().len(), expected);
    }
}

#[test]
fn t_release_does_not_change_track_count() {
    let patches = one_patch();
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    let mut release = press(KeyCode::Char('t'));
    release.kind = KeyEventKind::Release;

    assert!(matches!(
        screen.handle_key(release, Instant::now(), &ready_ctx(&patches)),
        GridSequencerAction::Continue
    ));
    assert_eq!(screen.track_count(), 4);
}

#[test]
fn help_opens_with_question_mark_and_closes_without_quitting() {
    let patches = one_patch();
    let mut screen = silent_screen();

    screen.handle_key(
        press(KeyCode::Char('?')),
        Instant::now(),
        &ready_ctx(&patches),
    );
    assert!(screen.help_open);

    // help 表示中の q は overlay を閉じるだけで、アプリを終了させない。
    assert!(matches!(
        screen.handle_key(
            press(KeyCode::Char('q')),
            Instant::now(),
            &ready_ctx(&patches)
        ),
        GridSequencerAction::Continue
    ));
    assert!(!screen.help_open);
}

#[test]
fn esc_closes_the_help_overlay() {
    let patches = one_patch();
    let mut screen = silent_screen();
    screen.handle_key(
        press(KeyCode::Char('?')),
        Instant::now(),
        &ready_ctx(&patches),
    );

    screen.handle_key(press(KeyCode::Esc), Instant::now(), &ready_ctx(&patches));

    assert!(!screen.help_open);
}

#[test]
fn r_assigns_a_patch_to_every_row() {
    let patches = one_patch();
    let mut screen = silent_screen();
    assert!(screen.state.rows().iter().all(|row| row.patch.is_none()));

    screen.handle_key(
        press(KeyCode::Char('r')),
        Instant::now(),
        &ready_ctx(&patches),
    );

    assert!(screen
        .state
        .rows()
        .iter()
        .all(|row| row.patch.as_deref() == Some("Keys/Piano.fxp")));
    assert!(screen
        .state
        .patches()
        .all(|patch| patch == Some("Keys/Piano.fxp")));
}

#[test]
fn r_keeps_the_patch_empty_while_the_list_is_still_loading() {
    let mut screen = silent_screen();
    let ctx = GridSequencerContext {
        patch_dirs_configured: true,
        patch_load: GridPatchLoad::Loading,
    };

    screen.handle_key(press(KeyCode::Char('r')), Instant::now(), &ctx);

    assert!(screen.state.rows().iter().all(|row| row.patch.is_none()));
}

/// SHIFT+R は音色ロード（＝無音時間）を避けるため patch を引き直さない。
#[test]
fn shift_r_rerolls_the_grid_without_touching_patches() {
    let patches = one_patch();
    let now = Instant::now();
    let mut screen = silent_screen();
    screen.start(now, &ready_ctx(&patches));
    for row in screen.state.rows_mut() {
        row.patch = Some("Kept/Patch.fxp".to_string());
        row.cells = [false; GRID_STEPS];
    }

    screen.handle_key(shift_press(KeyCode::Char('R')), now, &ready_ctx(&patches));

    assert!(screen
        .state
        .rows()
        .iter()
        .all(|row| row.patch.as_deref() == Some("Kept/Patch.fxp")));
    assert!(
        screen
            .state
            .rows()
            .iter()
            .any(|row| row.cells.iter().any(|cell| *cell)),
        "patch 以外は引き直すので、セルはどこかが note on になる"
    );
}

#[test]
fn ready_patch_list_fills_rows_that_started_while_loading() {
    let mut screen = silent_screen();
    let loading = GridSequencerContext {
        patch_dirs_configured: true,
        patch_load: GridPatchLoad::Loading,
    };
    screen.start(Instant::now(), &loading);
    assert!(screen.state.rows().iter().all(|row| row.patch.is_none()));

    let patches = one_patch();
    screen.refresh_context(&ready_ctx(&patches));

    assert!(screen
        .state
        .rows()
        .iter()
        .all(|row| row.patch.as_deref() == Some("Keys/Piano.fxp")));
    assert_eq!(screen.patch_status, GridPatchStatus::Ready(1));
}

#[test]
fn entering_the_screen_randomizes_and_starts_playing() {
    let patches = one_patch();
    let mut screen = silent_screen();

    screen.start(Instant::now(), &ready_ctx(&patches));

    assert!(screen.state.is_running());
    assert!(
        screen
            .state
            .rows()
            .iter()
            .any(|row| row.cells.iter().any(|cell| *cell)),
        "入った瞬間から鳴らすため、少なくとも1つのセルは note on になる"
    );
}

#[test]
fn leaving_the_screen_stops_the_clock_and_rewinds() {
    let patches = one_patch();
    let now = Instant::now();
    let mut screen = silent_screen();
    screen.start(now, &ready_ctx(&patches));

    screen.finish();

    assert!(!screen.state.is_running());
    assert_eq!(screen.state.step_index(), 0);
    assert!(!screen.help_open);
}

#[test]
fn resume_keeps_the_grid_and_restarts_the_clock() {
    let patches = one_patch();
    let now = Instant::now();
    let mut screen = silent_screen();
    screen.start(now, &ready_ctx(&patches));
    let grid = screen.state.rows().to_vec();
    screen.finish();

    screen.resume(now + STEP_INTERVAL);

    assert_eq!(screen.state.rows(), grid.as_slice());
    assert!(screen.state.is_running());
}

#[test]
fn entering_twice_keeps_the_grid_from_the_first_visit() {
    let patches = one_patch();
    let now = Instant::now();
    let mut screen = silent_screen();

    screen.enter(now, &ready_ctx(&patches));
    let first_grid = screen.state.rows().to_vec();
    screen.finish();
    screen.enter(now + STEP_INTERVAL, &ready_ctx(&patches));

    assert_eq!(screen.state.rows(), first_grid.as_slice());
    assert!(screen.state.is_running());
}

#[test]
fn key_release_events_are_ignored() {
    let patches = one_patch();
    let mut screen = silent_screen();
    let mut release = press(KeyCode::Char('q'));
    release.kind = KeyEventKind::Release;

    assert!(matches!(
        screen.handle_key(release, Instant::now(), &ready_ctx(&patches)),
        GridSequencerAction::Continue
    ));
}

/// 接続前に進めてしまうと、Ready 復帰時に欠落ステップをまとめて鳴らしてしまう。
#[test]
fn pump_step_does_not_advance_while_the_connection_is_not_ready() {
    let patches = one_patch();
    let now = Instant::now();
    let mut screen = silent_screen();
    screen.start(now, &ready_ctx(&patches));

    for step in 0..5u32 {
        screen.pump_step(now + STEP_INTERVAL * step);
    }

    assert_eq!(screen.state.step_index(), 0);
}
