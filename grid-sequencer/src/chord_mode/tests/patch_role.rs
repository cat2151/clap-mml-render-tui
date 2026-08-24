//! 最新のcatalog RoleをGrid用途へ写す経路の検証。

use super::*;

#[test]
fn chord_candidates_are_chord_role_and_poly_only() {
    let catalog = catalog();
    let patches = categorized_patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let screen = screen();

    let candidates = screen.patch_candidates_for_purpose(crate::GridPatchPurpose::Chord, &ctx);

    assert_eq!(
        candidates,
        vec![
            "patches_factory/Keys/Grand.fxp",
            "patches_factory/Pads/Warm.fxp",
            "patches_3rdparty/Vendor/Organs/Drawbar.fxp",
        ]
    );
}

#[test]
fn a_catalog_without_a_chord_role_patch_rejects_chord_mode() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = vec![("Basses/Sub.fxp".to_string(), "basses/sub.fxp".to_string())];
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen();
    screen.start(now, &ctx);

    screen.handle_key(press_c(), now, &ctx);

    assert!(screen.state.chord().is_none());
    assert_eq!(screen.chord_error(), Some(CHORD_PATCH_UNAVAILABLE));
}

#[test]
fn bass_and_arpeggio_rows_use_bass_and_lead_roles() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = vec![
        ("Keys/Poly.fxp".to_string(), "keys/poly.fxp".to_string()),
        ("Basses/Sub.fxp".to_string(), "basses/sub.fxp".to_string()),
        ("Leads/Mono.fxp".to_string(), "leads/mono.fxp".to_string()),
    ];
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);

    screen.handle_key(press_c(), now, &ctx);

    assert_eq!(
        screen.state.instances()[crate::BASS_ROW].patch.as_deref(),
        Some("Basses/Sub.fxp")
    );
    assert_eq!(
        screen.state.instances()[crate::ARPEGGIO_ROW]
            .patch
            .as_deref(),
        Some("Leads/Mono.fxp")
    );
}

#[test]
fn staged_cycle_reassigns_all_rows_from_their_purpose_pools() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = vec![
        ("Keys/Poly.fxp".to_string(), "keys/poly.fxp".to_string()),
        ("Basses/Sub.fxp".to_string(), "basses/sub.fxp".to_string()),
        ("Leads/Mono.fxp".to_string(), "leads/mono.fxp".to_string()),
        ("Synth/Other.fxp".to_string(), "synth/other.fxp".to_string()),
    ];
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
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
