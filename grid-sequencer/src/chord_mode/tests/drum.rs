//! drum 行の patch 抽選。役割ごとのカテゴリ・キーワードから引き直す経路。

use super::*;

/// drum 4 役を分けられるだけの候補。カテゴリは1つで、役割はキーワードで分かれる。
fn drum_patches() -> Vec<(String, String)> {
    vec![
        (
            "Percussion/Kick 909ish.fxp".to_string(),
            "percussion/kick 909ish.fxp".to_string(),
        ),
        (
            "Percussion/Snare Tight.fxp".to_string(),
            "percussion/snare tight.fxp".to_string(),
        ),
        (
            "Percussion/Closed Hi-Hat.fxp".to_string(),
            "percussion/closed hi-hat.fxp".to_string(),
        ),
        (
            "Percussion/Cowbell.fxp".to_string(),
            "percussion/cowbell.fxp".to_string(),
        ),
        ("Keys/Poly.fxp".to_string(), "keys/poly.fxp".to_string()),
    ]
}

/// drum 4 役をキーワードで分けるテスト用カタログ。ctx より先に束縛すること。
fn drum_plugins() -> PatchPlugins {
    plugins_with(PatchRoles {
        drum_patch_categories: vec!["Percussion".to_string()],
        kick_patch_keywords: vec!["kick".to_string()],
        snare_patch_keywords: vec!["snare".to_string()],
        hihat_patch_keywords: vec!["hat".to_string()],
        ..PatchRoles::default()
    })
}

/// drum 行は chord mode を使わなくても役割に合う patch が当たる。
/// 無差別抽選のままだと kick 行から pad が鳴る。
#[test]
fn drum_rows_get_their_own_patch_without_the_chord_mode() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = drum_patches();
    let plugins = drum_plugins();
    let mut ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllUnknown);
    ctx.patch_plugins = &plugins;
    let mut screen = GridSequencerScreen::with_track_count(None, crate::FULL_DRUM_TRACK_COUNT);

    screen.start(now, &ctx);

    let rows = screen.state.rows();
    assert!(screen.state.chord().is_none());
    assert_eq!(
        rows[crate::FULL_DRUM_TRACK_COUNT - 1].patch.as_deref(),
        Some("Percussion/Kick 909ish.fxp")
    );
    assert_eq!(
        rows[crate::FULL_DRUM_TRACK_COUNT - 2].patch.as_deref(),
        Some("Percussion/Snare Tight.fxp")
    );
    assert_eq!(
        rows[crate::FULL_DRUM_TRACK_COUNT - 3].patch.as_deref(),
        Some("Percussion/Closed Hi-Hat.fxp")
    );
    // percussion は他3役に取られなかった残り。
    assert_eq!(
        rows[crate::FIRST_DRUM_ROW].patch.as_deref(),
        Some("Percussion/Cowbell.fxp")
    );
}

/// `r` の無差別抽選のあとも、drum 行だけは役割に合う patch へ当て直す。
#[test]
fn randomizing_keeps_the_drum_rows_on_drum_patches() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = drum_patches();
    let plugins = drum_plugins();
    let mut ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllUnknown);
    ctx.patch_plugins = &plugins;
    let mut screen = GridSequencerScreen::with_track_count(None, crate::FULL_DRUM_TRACK_COUNT);
    screen.start(now, &ctx);

    screen.handle_key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
        now,
        &ctx,
    );

    assert_eq!(
        screen.state.rows()[crate::FULL_DRUM_TRACK_COUNT - 1]
            .patch
            .as_deref(),
        Some("Percussion/Kick 909ish.fxp")
    );
}

/// PATCH が ON のサイクル抽選でも drum 行を当て直す。
#[test]
fn the_staged_cycle_reassigns_the_drum_rows() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = drum_patches();
    let plugins = drum_plugins();
    let mut ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    ctx.patch_plugins = &plugins;
    let mut screen = GridSequencerScreen::with_track_count(None, crate::FULL_DRUM_TRACK_COUNT);
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);

    assert!(screen.stage_next_cycle(now, &ctx));

    let staged = screen.state.pending_rows_for_test();
    assert_eq!(
        staged[crate::FULL_DRUM_TRACK_COUNT - 1].patch.as_deref(),
        Some("Percussion/Kick 909ish.fxp")
    );
}
