use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::{LinePerformance, PatchCatalogEntry};

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

fn opened(patches: &[&str]) -> PatchAuditionSelect<'static> {
    let mut select = PatchAuditionSelect::default();
    select.open(PatchAuditionContext {
        catalog: ready(patches),
        ..Default::default()
    });
    select.request_select();
    select
}

#[test]
fn moving_the_cursor_asks_the_owner_what_to_audition_with_the_new_patch() {
    let mut select = opened(&["A.fxp", "B.fxp"]);

    let outcome = select.handle_select_key(press(KeyCode::Down));

    assert!(matches!(
        outcome,
        PatchSelectOutcome::Audition {
            moment: AuditionMoment::Candidate,
            patch: PatchChange::Switch(Some(ref patch)),
        } if patch == "B.fxp"
    ));
}

#[test]
fn a_line_audition_carries_the_play_settings() {
    let mut select = opened(&["A.fxp"]);
    select.set_play_settings(PlaySettings {
        repeat: true,
        ..Default::default()
    });

    let action = select.audition_action(
        Some(PatchAudition::Line(LinePerformance::silent())),
        PatchChange::Keep,
    );

    let PatchAuditionAction::PlayLine { program, .. } = action else {
        panic!("行は PlayLine になる");
    };
    assert!(program.repeat);
}

#[test]
fn a_loading_catalog_opens_the_select_once_it_is_synced() {
    let mut select = PatchAuditionSelect::default();
    select.open(PatchAuditionContext::default());
    select.request_select();
    assert_eq!(select.notice(), Some(&PatchCatalogNotice::Loading));

    select.sync_catalog(ready(&["A.fxp"]), Default::default(), Default::default());

    assert!(select.is_select_open());
    assert_eq!(select.notice(), None);
}

#[test]
fn reopening_keeps_the_patch() {
    let mut select = opened(&["A.fxp"]);
    select.set_patch(Some("A.fxp".to_string()));

    select.open(PatchAuditionContext::default());

    assert!(!select.is_select_open());
    assert_eq!(select.patch(), Some("A.fxp"));
}
