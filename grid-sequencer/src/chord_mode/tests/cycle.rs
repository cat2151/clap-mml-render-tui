//! 1周ごとの次サイクル抽選（[`crate::CycleRandom`]）の振る舞い。
//!
//! 「何を引き直すか」を項目ごとに切り替えたとき、差し替え待ちへ何が載るかを見る。

use super::*;

/// 進行を1周したら、進行・Key に加えて全行の音色も引き直す。
#[test]
fn staging_the_next_cycle_rerolls_every_patch() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = categorized_patches();
    let categories = ["Keys".to_string(), "Organs".to_string()];
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &categories);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);
    for row in screen.state.rows_mut() {
        row.patch = Some("Stale/Patch.fxp".to_string());
    }

    assert!(screen.stage_next_cycle(now, &ctx));

    // 鳴っている grid はそのまま。引き直しは差し替え待ちの側にだけ載る。
    assert!(
        screen
            .state
            .rows()
            .iter()
            .all(|row| row.patch.as_deref() == Some("Stale/Patch.fxp")),
        "演奏中の grid は触らない"
    );
    let staged = screen.state.pending_rows_for_test();
    assert!(
        staged
            .iter()
            .all(|row| row.patch.as_deref() != Some("Stale/Patch.fxp")),
        "全行の音色が引き直される"
    );
    let chord_patch = staged[CHORD_ROW].patch.clone().unwrap();
    assert!(
        chord_patch.contains("/Keys/") || chord_patch.contains("/Organs/"),
        "和音の行は対象カテゴリのまま: {chord_patch}"
    );
    assert!(screen.state.chord().is_some());
}

#[test]
fn holding_the_score_stages_a_new_progression_without_replacing_it() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = categorized_patches();
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &[]);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);
    screen.cycle_random = crate::CycleRandom::HOLD;
    for (index, row) in screen.state.rows_mut().iter_mut().enumerate() {
        row.patch = Some(format!("Kept/{index}.fxp"));
        row.base_note = 48 + index as u8;
        row.pattern = crate::NotePattern::from_steps((0..crate::GRID_STEPS).map(|step| {
            if step % (index + 2) == 0 {
                crate::NoteStep::Attack
            } else {
                crate::NoteStep::Rest
            }
        }));
    }
    let before = screen.state.rows().to_vec();

    assert!(screen.stage_next_cycle(now, &ctx));

    let staged = screen.state.pending_rows_for_test();
    for (actual, expected) in staged.iter().zip(&before) {
        assert_eq!(actual.patch, expected.patch);
        assert_eq!(actual.base_note, expected.base_note);
        assert_eq!(actual.pattern, expected.pattern);
    }
}

/// PATCH だけ ON。音色は毎周引き直され、譜面はそのまま残る。
#[test]
fn staging_with_only_the_patch_item_leaves_the_score_alone() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = categorized_patches();
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &[]);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);
    screen.cycle_random = crate::CycleRandom {
        patch: true,
        ..crate::CycleRandom::NONE
    };
    for row in screen.state.rows_mut() {
        row.patch = Some("Stale/Patch.fxp".to_string());
    }
    let before = screen.state.rows().to_vec();

    assert!(screen.stage_next_cycle(now, &ctx));

    let staged = screen.state.pending_rows_for_test();
    assert!(
        staged
            .iter()
            .all(|row| row.patch.as_deref() != Some("Stale/Patch.fxp")),
        "音色は引き直す"
    );
    for (staged, before) in staged.iter().zip(&before) {
        assert_eq!(staged.lanes, before.lanes, "譜面は据え置く");
    }
}

/// CHORD を OFF にした周は、同じ進行を頭から鳴らし直す。
#[test]
fn staging_without_the_chord_item_replays_the_same_progression_from_the_top() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = categorized_patches();
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &[]);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);
    screen.cycle_random = crate::CycleRandom {
        chord: false,
        ..crate::CycleRandom::ALL
    };
    let current = screen.state.chord().expect("chord mode が on").clone();

    assert!(screen.stage_next_cycle(now, &ctx));

    let staged = screen
        .state
        .pending_chord_for_test()
        .expect("次サイクルを預けている");
    assert_eq!(staged.degrees(), current.degrees());
    assert_eq!(staged.key(), current.key());
    assert_eq!(staged.index(), 0, "先頭のコードから鳴らし直す");
}

/// 何も引き直さない設定では、差し替えそのものを預けない。
#[test]
fn staging_nothing_leaves_the_current_cycle_in_place() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = categorized_patches();
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &[]);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);
    screen.cycle_random = crate::CycleRandom::NONE;

    assert!(screen.stage_next_cycle(now, &ctx));

    assert!(!screen.state.has_pending_cycle());
}
