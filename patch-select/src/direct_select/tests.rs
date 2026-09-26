use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::{CursorNotes, PatchCatalogEntry};

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ready(patches: &[&str]) -> PatchCatalogSnapshot {
    PatchCatalogSnapshot::Ready(
        patches
            .iter()
            .map(|patch| PatchCatalogEntry::from_display((*patch).to_string()))
            .collect(),
    )
}

fn note() -> PatchAudition {
    PatchAudition::Notes(CursorNotes {
        span: 0..1,
        pitches: vec![36],
        from_chord: false,
        velocity: 100,
        duration: std::time::Duration::from_millis(300),
    })
}

fn request(catalog: PatchCatalogSnapshot) -> DirectPatchSelectRequest {
    DirectPatchSelectRequest {
        context: PatchAuditionContext {
            catalog,
            ..Default::default()
        },
        patch: Some("A.fxp".to_string()),
        play_settings: PlaySettings::default(),
        audition: Some(note()),
    }
}

fn sets_patch(action: &PatchAuditionAction, expected: &str) -> bool {
    matches!(
        action,
        PatchAuditionAction::SetPatch { patch: Some(patch), notes: Some(_) } if patch == expected
    )
}

#[test]
fn opening_auditions_the_host_audition_with_the_current_patch() {
    let (select, opening) = DirectPatchSelect::open(request(ready(&["A.fxp", "B.fxp"])));

    assert!(select.is_select_open());
    let opening = opening.expect("開いた直後の試聴");
    assert!(sets_patch(&opening, "A.fxp"), "{opening:?}");
}

#[test]
fn moving_the_cursor_auditions_the_same_host_audition_with_the_candidate() {
    let (mut select, _) = DirectPatchSelect::open(request(ready(&["A.fxp", "B.fxp"])));

    let outcome = select.handle_key(press(KeyCode::Down));

    let DirectSelectOutcome::Play(action) = outcome else {
        panic!("候補移動は試聴: {outcome:?}");
    };
    assert!(sets_patch(&action, "B.fxp"), "{action:?}");
    assert_eq!(select.patch(), Some("A.fxp"));
}

#[test]
fn enter_confirms_and_closes_with_the_candidate_as_the_patch() {
    let (mut select, _) = DirectPatchSelect::open(request(ready(&["A.fxp", "B.fxp"])));
    select.handle_key(press(KeyCode::Down));

    let outcome = select.handle_key(press(KeyCode::Enter));

    assert_eq!(
        outcome,
        DirectSelectOutcome::Closed {
            confirmed: true,
            restore: None
        }
    );
    assert_eq!(select.patch(), Some("B.fxp"));
}

#[test]
fn esc_after_previewing_another_candidate_restores_the_original_patch() {
    let (mut select, _) = DirectPatchSelect::open(request(ready(&["A.fxp", "B.fxp"])));
    select.handle_key(press(KeyCode::Down));

    let outcome = select.handle_key(press(KeyCode::Esc));

    let DirectSelectOutcome::Closed {
        confirmed: false,
        restore: Some(PatchAuditionAction::SetPatch { patch, notes: None }),
    } = outcome
    else {
        panic!("取り消しは元の音色へ戻す: {outcome:?}");
    };
    assert_eq!(patch.as_deref(), Some("A.fxp"));
}

#[test]
fn a_loading_catalog_still_auditions_and_only_esc_closes() {
    let (mut select, opening) = DirectPatchSelect::open(request(PatchCatalogSnapshot::Loading));
    assert!(sets_patch(&opening.expect("待つ間も試聴する"), "A.fxp"));
    assert!(select.is_waiting_for_catalog());

    assert_eq!(
        select.handle_key(press(KeyCode::Char('x'))),
        DirectSelectOutcome::Continue
    );
    select.sync_catalog(ready(&["A.fxp"]), Default::default(), Default::default());
    assert!(select.is_select_open());
}

#[test]
fn esc_while_waiting_for_the_catalog_closes() {
    let (mut select, _) = DirectPatchSelect::open(request(PatchCatalogSnapshot::Loading));

    assert_eq!(
        select.handle_key(press(KeyCode::Esc)),
        DirectSelectOutcome::Closed {
            confirmed: false,
            restore: None
        }
    );
}

#[test]
fn an_empty_catalog_does_not_open() {
    let (mut select, opening) = DirectPatchSelect::open(request(ready(&[])));

    assert_eq!(opening, None);
    assert_eq!(select.notice(), Some(&PatchCatalogNotice::Empty));
    assert!(matches!(
        select.handle_key(press(KeyCode::Esc)),
        DirectSelectOutcome::Closed { .. }
    ));
}
