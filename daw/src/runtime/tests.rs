use super::*;
use crate::WorkspaceKind;

#[test]
fn daily_and_persistent_are_distinct_daw_screen_exit_targets() {
    assert!(!target_leaves_workspace(
        WorkspaceKind::Daily,
        PrimaryScreen::DailyDaw
    ));
    assert!(target_leaves_workspace(
        WorkspaceKind::Daily,
        PrimaryScreen::Daw
    ));
    assert!(target_leaves_workspace(
        WorkspaceKind::Daily,
        PrimaryScreen::Notepad
    ));
    assert!(!target_leaves_workspace(
        WorkspaceKind::Persistent,
        PrimaryScreen::Daw
    ));
    assert!(target_leaves_workspace(
        WorkspaceKind::Persistent,
        PrimaryScreen::DailyDaw
    ));
}
