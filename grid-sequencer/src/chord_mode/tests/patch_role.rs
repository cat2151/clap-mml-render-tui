//! 用途ごとの patch 抽選。カテゴリ設定で候補が絞られることの検証。

use super::*;

#[test]
fn the_chord_patch_is_limited_to_the_configured_categories() {
    let catalog = catalog();
    let patches = categorized_patches();
    let categories = ["Keys".to_string(), "Organs".to_string()];
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &categories);

    // 抽選なので、何回引いても対象カテゴリから出ないことを確かめる。
    for _ in 0..40 {
        let picked = pick_for_role(
            ctx.patches(),
            &ctx.role_filter(PatchRole::Chord).filter(),
            &ctx.poly_lookup(),
        )
        .expect("Keys / Organs は候補にある");
        assert!(
            picked.contains("/Keys/") || picked.contains("/Organs/"),
            "対象外のカテゴリを引いた: {picked}"
        );
    }
}

#[test]
fn an_empty_category_list_means_no_category_filter() {
    let catalog = catalog();
    let patches = categorized_patches();
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &[]);

    let mut seen = std::collections::HashSet::new();
    for _ in 0..200 {
        seen.insert(
            pick_for_role(
                ctx.patches(),
                &ctx.role_filter(PatchRole::Chord).filter(),
                &ctx.poly_lookup(),
            )
            .unwrap(),
        );
    }
    assert_eq!(seen.len(), patches.len(), "全カテゴリが当たりになる");
}

#[test]
fn a_category_with_no_poly_patch_yields_nothing() {
    let catalog = catalog();
    let patches = categorized_patches();
    let categories = ["Basses".to_string()];
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &categories);
    let mut screen = screen();
    screen.start(Instant::now(), &ctx);

    screen.handle_key(press_c(), Instant::now(), &ctx);

    assert!(screen.state.chord().is_none());
    assert_eq!(screen.chord_error(), Some(CHORD_PATCH_UNAVAILABLE));
}

/// bass 行には bass 用カテゴリの patch を当てる。poly でなくてよい。
#[test]
fn the_bass_row_gets_a_patch_from_the_bass_categories() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let bass_categories = vec!["Leads".to_string()];
    let mut ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    ctx.bass_patch_categories = &bass_categories;
    let mut screen = screen();
    screen.start(now, &ctx);

    screen.handle_key(press_c(), now, &ctx);

    assert_eq!(
        screen.state.instances()[crate::BASS_ROW].patch.as_deref(),
        Some("Leads/Mono.fxp"),
        "mono patch でも bass 行には使える"
    );
}

/// アルペジオ行は専用カテゴリからだけ引く。chord 行の候補集合とは独立。
#[test]
fn the_arpeggio_row_gets_a_patch_from_the_arpeggio_categories() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = vec![
        ("Keys/Poly.fxp".to_string(), "keys/poly.fxp".to_string()),
        ("Leads/Mono.fxp".to_string(), "leads/mono.fxp".to_string()),
        (
            "Percussion/Kick.fxp".to_string(),
            "percussion/kick.fxp".to_string(),
        ),
    ];
    let arpeggio_categories = vec!["Leads".to_string()];
    let mut ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    ctx.arpeggio_patch_categories = &arpeggio_categories;
    let mut screen = screen();
    screen.start(now, &ctx);

    screen.handle_key(press_c(), now, &ctx);

    assert_eq!(
        screen.state.instances()[crate::ARPEGGIO_ROW]
            .patch
            .as_deref(),
        Some("Leads/Mono.fxp"),
        "打楽器や chord 用 patch ではなく、arpeggio カテゴリから引く"
    );
}

/// PATCH が ON のサイクル抽選でも、用途の決まった3行は専用カテゴリへ当て直す。
#[test]
fn the_staged_cycle_reassigns_the_dedicated_rows_from_their_categories() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = vec![
        ("Keys/Poly.fxp".to_string(), "keys/poly.fxp".to_string()),
        ("Leads/Mono.fxp".to_string(), "leads/mono.fxp".to_string()),
        ("Basses/Sub.fxp".to_string(), "basses/sub.fxp".to_string()),
    ];
    let chord_categories = vec!["Keys".to_string()];
    let bass_categories = vec!["Basses".to_string()];
    let arpeggio_categories = vec!["Leads".to_string()];
    let mut ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    ctx.chord_patch_categories = &chord_categories;
    ctx.bass_patch_categories = &bass_categories;
    ctx.arpeggio_patch_categories = &arpeggio_categories;
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);

    assert!(screen.stage_next_cycle(now, &ctx));

    let staged = screen.state.pending_rows_for_test();
    assert_eq!(staged[CHORD_ROW].patch.as_deref(), Some("Keys/Poly.fxp"));
    assert_eq!(
        staged[crate::BASS_ROW].patch.as_deref(),
        Some("Basses/Sub.fxp")
    );
    assert_eq!(
        staged[crate::ARPEGGIO_ROW].patch.as_deref(),
        Some("Leads/Mono.fxp")
    );
}
