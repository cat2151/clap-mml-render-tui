//! chord mode の永続化（セッションからの復元と、`t` キーをまたぐ持ち越し）。

use super::*;
use crate::{FixedChordProgression, GridInstance, GridSequencerParts, GridSequencerSession};

fn press_t() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)
}

/// 前回 chord mode が on だったセッションから復元した画面。
fn restored_screen() -> GridSequencerScreen {
    GridSequencerScreen::new_with(GridSequencerParts {
        chord_enabled: true,
        ..GridSequencerParts::default()
    })
}

fn restored_fixed_screen(input: &str) -> GridSequencerScreen {
    let mut cycle_random = crate::CycleRandom::ALL;
    cycle_random.chord = false;
    let session = GridSequencerSession::new(vec![GridInstance::new(0)], cycle_random)
        .with_fixed_chord(Some(FixedChordProgression::new(input)));
    GridSequencerScreen::new_with(GridSequencerParts {
        chord_enabled: true,
        restored_session: Some(session),
        ..GridSequencerParts::default()
    })
}

#[test]
fn the_saved_flag_follows_the_chord_mode() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);
    assert!(!screen.chord_enabled());

    screen.handle_key(press_c(), now, &ctx);
    assert!(screen.chord_enabled());

    screen.handle_key(press_c(), now, &ctx);
    assert!(!screen.chord_enabled());
}

/// `t` は `state` を作り直すので、chord mode を `GridState` に持たせると必ず落ちる。
#[test]
fn a_track_count_change_keeps_the_saved_chord_mode() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);

    screen.handle_key(press_t(), now, &ctx);

    assert!(screen.state.chord().is_none(), "grid ごと作り直される");
    assert!(
        screen.chord_enabled(),
        "再起動後のセッションへ on のまま渡す"
    );
}

#[test]
fn a_restored_chord_mode_waits_for_the_patch_list_then_turns_on() {
    let catalog = catalog();
    let patches = patches();
    let loading = ctx_with(GridPatchLoad::Loading, &catalog, &OnePolyPatch);
    let mut screen = restored_screen();
    screen.start(Instant::now(), &loading);

    screen.refresh_context(&loading);
    assert!(screen.state.chord().is_none(), "読み込み中は待つ");
    assert!(screen.chord_error().is_none(), "待ちは理由を出さない");

    let ready = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    screen.refresh_context(&ready);

    assert!(screen.state.chord().is_some());
    assert!(screen.chord_enabled());
    assert_eq!(
        screen.state.rows()[CHORD_ROW].patch.as_deref(),
        Some("Keys/Poly.fxp"),
    );
}

/// 復元に失敗したら理由を出して諦める。毎フレーム引き直すと log が溢れるだけ。
#[test]
fn a_restored_chord_mode_is_not_retried_after_it_fails() {
    let empty = ChordProgressionCatalog::default();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &empty, &OnePolyPatch);
    let mut screen = restored_screen();
    screen.start(Instant::now(), &ctx);

    screen.refresh_context(&ctx);

    assert!(screen.state.chord().is_none());
    assert_eq!(screen.chord_error(), Some(CATALOG_UNAVAILABLE));
    assert!(!screen.pending_chord, "予約は1回で下ろす");
    assert!(
        screen.chord_enabled(),
        "失敗は一時的なこともあるので、ユーザーの選択は保持したまま次回また試す"
    );
}

#[test]
fn a_saved_fixed_progression_is_restored_without_the_catalog() {
    let empty = ChordProgressionCatalog::default();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &empty, &OnePolyPatch);
    let mut screen = restored_fixed_screen("key:G Isus4-I");

    screen.start(Instant::now(), &ctx);
    screen.refresh_context(&ctx);

    let chord = screen.state.chord().expect("固定進行を復元する");
    assert_eq!((chord.key(), chord.degrees()), ("G", "Isus4-I"));
    assert_eq!(screen.fixed_chord().unwrap().input(), "key:G Isus4-I");
    assert!(!screen.cycle_random().chord);
}

#[test]
fn invalid_saved_fixed_text_is_preserved_as_an_editable_error() {
    let patches = patches();
    let empty = ChordProgressionCatalog::default();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &empty, &OnePolyPatch);
    let mut screen = restored_fixed_screen("not valid yet");

    screen.start(Instant::now(), &ctx);
    screen.refresh_context(&ctx);

    assert!(screen.state.chord().is_none());
    assert!(screen.chord_error().is_some());
    assert_eq!(screen.fixed_chord().unwrap().input(), "not valid yet");
    screen.open_chord_input();
    assert_eq!(
        screen.chord_input_overlay().unwrap().value(),
        "not valid yet"
    );
}
