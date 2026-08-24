use crossterm::event::KeyModifiers;

use super::*;
use std::borrow::Cow;

use cmrt_patches::{PatchRoleIndex, PatchRoleInput};

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

/// chord mode を使わないテスト用の空カタログ。
pub(crate) fn empty_catalog() -> &'static cmrt_chord::ChordProgressionCatalog {
    static CATALOG: std::sync::OnceLock<cmrt_chord::ChordProgressionCatalog> =
        std::sync::OnceLock::new();
    CATALOG.get_or_init(cmrt_chord::ChordProgressionCatalog::default)
}

pub(crate) fn patch_roles(
    patches: &[(String, String)],
    user_presets: &[(String, String)],
) -> PatchRoleIndex {
    PatchRoleIndex::build(
        patches
            .iter()
            .map(|(display, normalized_display)| PatchRoleInput {
                display,
                normalized_display,
                selector_category: None,
            }),
        user_presets,
    )
}

pub(crate) fn ctx_with<'a>(
    patch_load: GridPatchLoad<'a>,
    catalog: &'a cmrt_chord::ChordProgressionCatalog,
    voicing: &'a dyn GridVoicingLookup,
) -> GridSequencerContext<'a> {
    let patch_roles = match &patch_load {
        GridPatchLoad::Ready(patches) => patch_roles(patches, &[]),
        GridPatchLoad::Loading | GridPatchLoad::Err(_) => PatchRoleIndex::default(),
    };
    GridSequencerContext {
        patch_dirs_configured: true,
        patch_load,
        load_measurements: None,
        chord_catalog: catalog,
        voicing,
        patch_roles: Cow::Owned(patch_roles),
        chord_source_updated: false,
        catalog_notes: &[],
    }
}

pub(crate) fn ready_ctx(patches: &[(String, String)]) -> GridSequencerContext<'_> {
    ctx_with(
        GridPatchLoad::Ready(patches),
        empty_catalog(),
        &NoVoicingLookup,
    )
}

fn loading_ctx() -> GridSequencerContext<'static> {
    ctx_with(GridPatchLoad::Loading, empty_catalog(), &NoVoicingLookup)
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

    for expected in [2, 3, 4, 7, 8, 16, 1] {
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
fn r_assigns_note_patches_but_does_not_fallback_for_empty_drum_pools() {
    let patches = one_patch();
    let mut screen = silent_screen();
    assert!(screen.state.rows().iter().all(|row| row.patch.is_none()));

    screen.handle_key(
        press(KeyCode::Char('r')),
        Instant::now(),
        &ready_ctx(&patches),
    );

    for (index, row) in screen.state.rows().iter().enumerate() {
        if screen.state.drum_role(index).is_some() {
            assert_eq!(row.patch, None, "drum候補が空ならALLへfallbackしない");
        } else {
            assert_eq!(
                row.patch.as_deref(),
                Some("Keys/Piano.fxp"),
                "non-drum row {index}"
            );
        }
    }
}

#[test]
fn r_keeps_the_patch_empty_while_the_list_is_still_loading() {
    let mut screen = silent_screen();

    screen.handle_key(press(KeyCode::Char('r')), Instant::now(), &loading_ctx());

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
        row.pattern = NotePattern::default();
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
            .any(|row| row.pattern.steps().contains(&NoteStep::Attack)),
        "patch 以外は引き直すので、どこかにAttackが生成される"
    );
}

#[test]
fn ready_patch_list_fills_rows_that_started_while_loading() {
    let mut screen = silent_screen();
    screen.start(Instant::now(), &loading_ctx());
    assert!(screen.state.rows().iter().all(|row| row.patch.is_none()));

    let patches = one_patch();
    screen.refresh_context(&ready_ctx(&patches));

    for (index, row) in screen.state.rows().iter().enumerate() {
        if screen.state.drum_role(index).is_some() {
            assert_eq!(row.patch, None);
        } else {
            assert_eq!(row.patch.as_deref(), Some("Keys/Piano.fxp"));
        }
    }
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
            .any(|row| row.pattern.steps().contains(&NoteStep::Attack)),
        "入った瞬間から鳴らすため、少なくとも1つのAttackが生成される"
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

    screen.resume(now + STEP_INTERVAL, &ready_ctx(&patches));

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

/// `a` は overlay の開閉だけ。設定は overlay の中で切り替える。
#[test]
fn a_opens_the_cycle_random_overlay_and_randomize_keeps_the_settings() {
    let patches = one_patch();
    let now = Instant::now();
    let mut screen = silent_screen();
    assert_eq!(screen.cycle_random(), crate::CycleRandom::ALL);

    screen.handle_key(press(KeyCode::Char('a')), now, &ready_ctx(&patches));
    assert!(screen.cycle_random_open());
    // overlay の `2` は NOTE。
    screen.handle_key(press(KeyCode::Char('2')), now, &ready_ctx(&patches));
    assert!(!screen.cycle_random().note);

    screen.handle_key(press(KeyCode::Esc), now, &ready_ctx(&patches));
    assert!(!screen.cycle_random_open());

    // `r` / `R` はその場の引き直しであって、1周ごとの設定には触らない。
    screen.handle_key(press(KeyCode::Char('r')), now, &ready_ctx(&patches));
    screen.handle_key(shift_press(KeyCode::Char('R')), now, &ready_ctx(&patches));
    assert!(!screen.cycle_random().note);
}

#[test]
fn x_clears_cells_keeps_row_parameters_and_stops_the_note_random() {
    let patches = one_patch();
    let mut screen = silent_screen();
    let row = &mut screen.state.rows_mut()[1];
    row.patch = Some("Kept/Patch.fxp".to_string());
    row.base_note = 73;
    row.pattern.draw_span(4, 7);

    screen.handle_key(
        press(KeyCode::Char('x')),
        Instant::now(),
        &ready_ctx(&patches),
    );

    let row = &screen.state.rows()[1];
    assert_eq!(row.patch.as_deref(), Some("Kept/Patch.fxp"));
    assert_eq!(row.base_note, 73);
    assert_eq!(row.pattern, NotePattern::default());
    assert!(!screen.cycle_random().get(crate::CycleRandomItem::Note));
}

/// DAW 画面の `P` と同じく、押すたびに停止と再開が入れ替わる。
#[test]
fn shift_p_toggles_playing() {
    let patches = one_patch();
    let now = Instant::now();
    let mut screen = silent_screen();
    screen.start(now, &ready_ctx(&patches));
    assert!(screen.is_playing());

    screen.handle_key(shift_press(KeyCode::Char('P')), now, &ready_ctx(&patches));
    assert!(!screen.is_playing());

    screen.handle_key(shift_press(KeyCode::Char('P')), now, &ready_ctx(&patches));
    assert!(screen.is_playing());
}

/// 止めても grid は残る。`P` で戻したときに同じ譜面が鳴り直すのが前提。
#[test]
fn stopping_keeps_the_grid_contents() {
    let patches = one_patch();
    let now = Instant::now();
    let mut screen = silent_screen();
    screen.start(now, &ready_ctx(&patches));
    let before = screen
        .state
        .rows()
        .iter()
        .map(|row| row.pattern.clone())
        .collect::<Vec<_>>();

    screen.stop_playing();

    let after = screen
        .state
        .rows()
        .iter()
        .map(|row| row.pattern.clone())
        .collect::<Vec<_>>();
    assert_eq!(before, after);
    assert!(!screen.is_playing());
}

/// `stop_playing` は演奏だけを止める。開いている入力欄まで閉じるのは
/// 画面を離れる `finish` の役目。
#[test]
fn stopping_does_not_close_the_chord_input() {
    let patches = one_patch();
    let now = Instant::now();
    let mut screen = silent_screen();
    screen.start(now, &ready_ctx(&patches));
    screen.handle_key(press(KeyCode::Char('i')), now, &ready_ctx(&patches));
    assert!(screen.chord_input_open());

    screen.stop_playing();
    assert!(screen.chord_input_open());

    screen.finish();
    assert!(!screen.chord_input_open());
}

/// 接続前に進めてしまうと、Ready 復帰時に欠落ステップをまとめて鳴らしてしまう。
#[test]
fn pump_step_does_not_advance_while_the_connection_is_not_ready() {
    let patches = one_patch();
    let now = Instant::now();
    let mut screen = silent_screen();
    screen.start(now, &ready_ctx(&patches));

    for step in 0..5u32 {
        screen.pump_step(now + STEP_INTERVAL * step, &ready_ctx(&patches));
    }

    assert_eq!(screen.state.step_index(), 0);
}
