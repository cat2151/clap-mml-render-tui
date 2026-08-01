use super::*;
use crate::playback::position::set_grid_for_test;

fn auto_random_browser() -> LoopBrowser {
    let mut browser = browser_with_direct_wavs(2);
    let first = browser.wav_analyses[0].0.clone();
    browser.track_grid = vec![vec![Some(LoopTrackClip::explicit(first, 1))]];
    browser
}

/// random deck の保存先。commit は deck の保存に成功して初めて表示を確定するので、
/// commit まで進めるテストでは実体のあるパスが要る。
fn enable_random_persistence(browser: &mut LoopBrowser, name: &str) -> PathBuf {
    let path = std::env::temp_dir()
        .join(format!(
            "cmrt-auto-random-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join("loop_browser")
        .join("random_decks.toml");
    browser.random_decks.path = Some(path.clone());
    path
}

fn playing(browser: &LoopBrowser, cycle: u64, token: u64) {
    set_grid_for_test(&browser.playback_position, cycle, token);
}

fn staged_token(action: &LoopBrowserAction) -> u64 {
    match action {
        LoopBrowserAction::GridPreload { token, .. } => *token,
        _ => panic!("GridPreload を期待したが別の action だった"),
    }
}

#[test]
fn shift_o_toggles_auto_random_from_both_panes_and_persists_it() {
    let mut browser = auto_random_browser();
    let dir = std::env::temp_dir().join(format!(
        "cmrt-auto-random-toggle-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = dir.join("loop_browser.toml");
    browser.metadata.path = Some(path.clone());

    assert!(!browser.auto_random());
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('O'), KeyModifiers::SHIFT));
    assert!(browser.auto_random());
    assert!(LoopBrowserMetadata::load_from(&path).unwrap().auto_random);

    browser.focus = LoopBrowserPane::Tracks;
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('O'), KeyModifiers::SHIFT));
    assert!(!browser.auto_random());
    assert!(!LoopBrowserMetadata::load_from(&path).unwrap().auto_random);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_next_grid_is_staged_on_the_last_cycle_and_only_once() {
    let mut browser = auto_random_browser();
    browser.metadata.value.auto_random = true;

    // 0 周目は先読みしない。最後の 1 周を丸ごと準備の猶予に使いたいので、
    // 早く引きすぎると差し替えまでの待ちが伸びるだけになる。
    playing(&browser, 0, 0);
    assert!(matches!(
        browser.pump_auto_random(),
        LoopBrowserAction::Continue
    ));

    playing(&browser, 1, 0);
    let token = staged_token(&browser.pump_auto_random());
    assert_eq!(token, 1);

    // 同じ周のあいだ毎フレーム呼ばれても二重に予約しない。
    assert!(matches!(
        browser.pump_auto_random(),
        LoopBrowserAction::Continue
    ));
}

#[test]
fn the_staged_grid_is_committed_only_once_it_is_actually_playing() {
    let mut browser = auto_random_browser();
    browser.metadata.value.auto_random = true;
    let deck_path = enable_random_persistence(&mut browser, "commit");
    let before = browser.track_grid.clone();

    playing(&browser, 1, 0);
    let token = staged_token(&browser.pump_auto_random());

    // まだ差し替わっていないので表示は変えない。
    playing(&browser, 1, 0);
    browser.pump_auto_random();
    assert_eq!(browser.track_grid, before);

    // worker が差し替えた token が返ってきて初めて表示を確定する。
    playing(&browser, 0, token);
    browser.pump_auto_random();
    assert_ne!(browser.track_grid, before);
    assert!(browser.track_grid_error.is_none());
    assert!(browser.random_decks.error.is_none());
    let _ = std::fs::remove_dir_all(deck_path.parent().unwrap().parent().unwrap());
}

#[test]
fn a_staged_grid_that_never_starts_playing_is_dropped_and_drawn_again() {
    let mut browser = auto_random_browser();
    browser.metadata.value.auto_random = true;

    playing(&browser, 1, 0);
    let first = staged_token(&browser.pump_auto_random());

    // 準備が間に合わず差し替わらないまま周が進んだら、予約を捨てて引き直す。
    playing(&browser, 3, 0);
    let second = staged_token(&browser.pump_auto_random());
    assert_ne!(first, second);
}

#[test]
fn nothing_is_staged_while_auto_random_is_off_or_playback_is_stopped() {
    let mut browser = auto_random_browser();

    playing(&browser, 1, 0);
    assert!(matches!(
        browser.pump_auto_random(),
        LoopBrowserAction::Continue
    ));

    // 停止中（再生位置なし）は周回数が読めないので何もしない。
    browser.metadata.value.auto_random = true;
    crate::playback::position::clear(&browser.playback_position);
    assert!(matches!(
        browser.pump_auto_random(),
        LoopBrowserAction::Continue
    ));
}
