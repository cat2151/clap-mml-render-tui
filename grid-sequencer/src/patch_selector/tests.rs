use std::time::Instant;

use cmrt_realtime_play::PatchVoicing;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::*;
use crate::{tests::ctx_with, ChordPlayback, GridPatchLoad, GridRow, PatternEvolution};

const AREA: Rect = Rect::new(0, 0, 100, 30);

fn patches() -> Vec<(String, String)> {
    [
        "Bass/Mono.fxp",
        "Keys/Alpha.fxp",
        "Keys/Beta.fxp",
        "Keys/Unknown.fxp",
        "Pads/Poly.fxp",
    ]
    .into_iter()
    .map(|patch| (patch.to_string(), patch.to_lowercase()))
    .collect()
}

struct Voicing;

impl crate::GridVoicingLookup for Voicing {
    fn cached_voicing(&self, patch: &str) -> Option<PatchVoicing> {
        match patch {
            "Keys/Alpha.fxp" | "Keys/Beta.fxp" | "Pads/Poly.fxp" => Some(PatchVoicing::Poly),
            "Bass/Mono.fxp" => Some(PatchVoicing::Mono),
            "Keys/Unknown.fxp" => Some(PatchVoicing::Unknown),
            _ => None,
        }
    }
}

fn context(patches: &[(String, String)]) -> GridSequencerContext<'_> {
    ctx_with(
        GridPatchLoad::Ready(patches),
        crate::tests::empty_catalog(),
        &Voicing,
    )
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn ready_catalog_opens_at_the_rows_current_patch() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.rows_mut()[1].patch = Some("Keys/Beta.fxp".to_string());

    screen.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), 7, 3),
        AREA,
        &ctx,
    );

    let selector = screen.patch_selector.as_ref().unwrap();
    assert_eq!(selector.instance, 1);
    assert_eq!(selector.selected_category().name, "Keys");
    assert_eq!(selector.selected_patch(), Some("Keys/Beta.fxp"));
}

#[test]
fn loading_error_empty_and_unconfigured_catalogs_do_not_open() {
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    let loading = ctx_with(
        GridPatchLoad::Loading,
        crate::tests::empty_catalog(),
        &Voicing,
    );
    screen.open_patch_selector(0, &loading);
    assert!(screen.patch_selector.is_none());

    let error = ctx_with(
        GridPatchLoad::Err("catalog failed"),
        crate::tests::empty_catalog(),
        &Voicing,
    );
    screen.open_patch_selector(0, &error);
    assert!(screen.patch_selector.is_none());

    let empty = context(&[]);
    screen.open_patch_selector(0, &empty);
    assert!(screen.patch_selector.is_none());

    let patches = patches();
    let mut unconfigured = context(&patches);
    unconfigured.patch_dirs_configured = false;
    screen.open_patch_selector(0, &unconfigured);
    assert!(screen.patch_selector.is_none());
}

#[test]
fn clicking_a_patch_changes_only_that_row_enters_hold_and_cancels_pending_cycle() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.state.rows_mut()[1].patch = Some("Bass/Mono.fxp".to_string());
    screen.state.stage_next_cycle(
        vec![GridRow::default(); 2],
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap(),
    );
    screen.open_patch_selector(0, &ctx);
    let layout = PatchSelectorLayout::new(AREA);
    let beta = screen
        .patch_selector
        .as_ref()
        .unwrap()
        .selected_category()
        .patches
        .iter()
        .position(|patch| patch == "Keys/Beta.fxp")
        .unwrap();

    screen.handle_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            layout.patch_list.x,
            layout.patch_list.y + beta as u16,
        ),
        AREA,
        &ctx,
    );

    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Beta.fxp")
    );
    assert_eq!(
        screen.state.rows()[1].patch.as_deref(),
        Some("Bass/Mono.fxp")
    );
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Hold);
    assert!(!screen.state.has_pending_cycle());
    assert!(screen.patch_selector.is_none());

    screen.handle_key(press(KeyCode::Char('u')), Instant::now(), &ctx);
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Auto);
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

    let selector = screen.patch_selector.as_ref().unwrap();
    let candidates = selector
        .categories
        .iter()
        .flat_map(|category| category.patches.iter())
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        candidates,
        vec!["Keys/Alpha.fxp", "Keys/Beta.fxp", "Pads/Poly.fxp"]
    );
}

#[test]
fn patch_name_wheel_uses_chord_candidates_only_on_the_chord_instance() {
    let patches = ["Bass/Mono.fxp", "Pads/Poly.fxp"]
        .into_iter()
        .map(|patch| (patch.to_string(), patch.to_lowercase()))
        .collect::<Vec<_>>();
    let chord_categories = vec!["Pads".to_string()];
    let bass_categories = vec!["Bass".to_string()];
    let mut ctx = context(&patches);
    ctx.chord_patch_categories = &chord_categories;
    ctx.bass_patch_categories = &bass_categories;
    // chord ON の行は 3=和音、4=bass、5〜8が 4 voice。
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.instances_mut()[0].patch = Some("Bass/Mono.fxp".to_string());
    screen.state.instances_mut()[1].patch = Some("Pads/Poly.fxp".to_string());
    screen.state.instances_mut()[2].patch = Some("Pads/Poly.fxp".to_string());
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    // chord summaryのPATCH wheelは、既存のカテゴリ+poly条件を満たす候補だけから選ぶ。
    screen.handle_mouse(mouse(MouseEventKind::ScrollDown, 7, 3), AREA, &ctx);
    assert_eq!(
        screen.state.instances()[0].patch.as_deref(),
        Some("Pads/Poly.fxp")
    );

    // bass 行は bass 用カテゴリから引く。poly 判定は問わない。
    screen.handle_mouse(mouse(MouseEventKind::ScrollDown, 7, 4), AREA, &ctx);
    assert_eq!(
        screen.state.instances()[1].patch.as_deref(),
        Some("Bass/Mono.fxp")
    );

    // 4 voiceのchild lane上でもinstance共有PATCHを変更し、chord候補は除外する。
    screen.handle_mouse(mouse(MouseEventKind::ScrollUp, 7, 6), AREA, &ctx);
    assert_eq!(
        screen.state.instances()[2].patch.as_deref(),
        Some("Bass/Mono.fxp")
    );
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Hold);
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
        mouse(MouseEventKind::Down(MouseButton::Left), 7, 6),
        AREA,
        &ctx,
    );

    let selector = screen.patch_selector.as_ref().unwrap();
    assert_eq!(selector.instance, 2);
    assert!(selector
        .categories
        .iter()
        .flat_map(|category| &category.patches)
        .any(|patch| patch == "Bass/Mono.fxp"));
    screen.patch_selector.as_mut().unwrap().category_cursor = 0;
    screen.patch_selector.as_mut().unwrap().patch_cursor = 0;
    assert_eq!(
        screen.patch_selector.as_ref().unwrap().selected_patch(),
        Some("Bass/Mono.fxp")
    );
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
    let error = ctx_with(
        GridPatchLoad::Err("catalog disappeared"),
        crate::tests::empty_catalog(),
        &Voicing,
    );

    screen.handle_patch_selector_key(press(KeyCode::Enter), &error);

    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Auto);
}

#[test]
fn mouse_wheel_previews_the_cursor_patch_without_committing_until_enter() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ctx);
    let layout = PatchSelectorLayout::new(AREA);

    screen.handle_patch_selector_mouse(
        mouse(
            MouseEventKind::ScrollDown,
            layout.patch_list.x,
            layout.patch_list.y,
        ),
        AREA,
        &ctx,
    );
    assert_eq!(
        screen.patch_selector.as_ref().unwrap().selected_patch(),
        Some("Keys/Beta.fxp")
    );
    assert_eq!(
        screen
            .patch_selector
            .as_ref()
            .unwrap()
            .previewed_patch
            .as_deref(),
        Some("Keys/Beta.fxp")
    );
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );

    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);
    assert_eq!(
        screen.state.rows()[0].patch.as_deref(),
        Some("Keys/Beta.fxp")
    );
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
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Hold);
}

#[test]
fn escape_cancels_a_preview_without_committing_it() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    screen.open_patch_selector(0, &ctx);

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
    assert_eq!(screen.pattern_evolution(), PatternEvolution::Auto);
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

/// アルペジオ行の PATCH wheel は arpeggio 用カテゴリからだけ引く。
/// chord mode 中は「chord 用カテゴリ以外」ではなくなるので、打楽器は候補に入らない。
#[test]
fn patch_name_wheel_uses_arpeggio_candidates_on_the_arpeggio_instance() {
    let patches = ["Percussion/Kick.fxp", "Leads/Mono.fxp", "Pads/Poly.fxp"]
        .into_iter()
        .map(|patch| (patch.to_string(), patch.to_lowercase()))
        .collect::<Vec<_>>();
    let arpeggio_categories = vec!["Leads".to_string()];
    let mut ctx = context(&patches);
    ctx.arpeggio_patch_categories = &arpeggio_categories;
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.instances_mut()[crate::ARPEGGIO_ROW].patch =
        Some("Percussion/Kick.fxp".to_string());
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    // 行3（4 voice）の PATCH 欄で wheel。行5〜8 のどの voice row からでも instance 単位。
    screen.handle_mouse(mouse(MouseEventKind::ScrollDown, 7, 6), AREA, &ctx);

    assert_eq!(
        screen.state.instances()[crate::ARPEGGIO_ROW]
            .patch
            .as_deref(),
        Some("Leads/Mono.fxp")
    );
}
