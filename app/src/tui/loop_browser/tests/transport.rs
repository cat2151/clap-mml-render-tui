use super::*;

#[test]
fn p_toggles_loop_playback_from_both_panes_and_resumes_at_selected_measure() {
    let mut browser = browser_with_direct_wavs(1);
    browser.measure_cursor = 3;

    assert!(matches!(
        browser.handle_key(KeyCode::Char('p')),
        LoopBrowserAction::SetPlaybackPaused {
            paused: true,
            start_measure: 3,
        }
    ));
    assert!(browser.playback_paused);

    browser.focus = LoopBrowserPane::Tracks;
    browser.measure_cursor = 5;
    assert!(matches!(
        browser.handle_key(KeyCode::Char('p')),
        LoopBrowserAction::SetPlaybackPaused {
            paused: false,
            start_measure: 5,
        }
    ));
    assert!(!browser.playback_paused);
}
