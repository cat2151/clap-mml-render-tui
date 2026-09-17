//! patch 一覧の読み込み状態と行の patch 割り当て。読み込み中は空のまま、揃ったら埋める。

use super::*;

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
