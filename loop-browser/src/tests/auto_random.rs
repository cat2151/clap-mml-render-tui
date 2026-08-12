use super::*;
use crate::playback::position::set_grid_for_test;

fn auto_random_browser() -> LoopBrowser {
    auto_random_browser_with_span(1)
}

/// 1 周が `span_measures` 小節のグリッド。ヘルパの WAV は BPM120 4/4 なので 1 小節 2 秒、
/// span 8 でちょうど 16 秒（「長いグリッド」の境界）になる。
fn auto_random_browser_with_span(span_measures: usize) -> LoopBrowser {
    let mut browser = browser_with_direct_wavs(2);
    let first = browser.wav_analyses[0].0.clone();
    browser.track_grid = vec![vec![Some(LoopTrackClip::explicit(first, span_measures))]];
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

fn staged_mode(action: &LoopBrowserAction) -> cmrt_tui_core::bpm::BpmMode {
    match action {
        LoopBrowserAction::GridPreload { mode, .. } => *mode,
        _ => panic!("GridPreload を期待したが別の action だった"),
    }
}

/// 自動BPMに幅を持たせた browser。ヘルパの WAV は BPM120 なので、100〜140 は
/// time stretch 範囲（96〜150）に丸ごと収まる＝引いた値がそのまま通る。
fn ranged_auto_random_browser() -> LoopBrowser {
    let mut browser = auto_random_browser();
    browser.metadata.value.auto_random = true;
    browser.set_bpm_range(cmrt_tui_core::bpm::BpmRange::new(100.0, 140.0).unwrap());
    browser.set_bpm_mode(cmrt_tui_core::bpm::BpmMode::Auto(120.0));
    browser
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
fn a_long_cycle_stages_the_next_grid_on_its_first_lap_instead_of_repeating_itself() {
    let mut browser = auto_random_browser_with_span(8);
    browser.metadata.value.auto_random = true;
    assert_eq!(browser.cycle_seconds(), 16.0);
    assert_eq!(browser.auto_random_cycles(), 1);

    // 16 秒あれば 1 周の途中でも先読みは間に合う。同じ内容を 2 周聴かせるほうが間延びする。
    playing(&browser, 0, 0);
    assert_eq!(staged_token(&browser.pump_auto_random()), 1);
}

#[test]
fn a_cycle_just_under_the_threshold_still_plays_two_laps() {
    let mut browser = auto_random_browser_with_span(7);
    browser.metadata.value.auto_random = true;
    assert_eq!(browser.cycle_seconds(), 14.0);
    assert_eq!(browser.auto_random_cycles(), 2);

    playing(&browser, 0, 0);
    assert!(matches!(
        browser.pump_auto_random(),
        LoopBrowserAction::Continue
    ));

    playing(&browser, 1, 0);
    assert_eq!(staged_token(&browser.pump_auto_random()), 1);
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
fn each_staged_cycle_redraws_the_automatic_bpm_without_moving_the_current_one() {
    let mut browser = ranged_auto_random_browser();
    let playing_mode = browser.bpm_mode();

    let mut drawn = std::collections::HashSet::new();
    // 予約が差し替わらないまま周が進むと捨てて引き直す。その周ごとにテンポも引き直す。
    for cycle in (1..48).step_by(2) {
        playing(&browser, cycle, 0);
        let mode = staged_mode(&browser.pump_auto_random());
        assert!(
            (100.0..=140.0).contains(&mode.bpm()),
            "範囲外を引いた: {}",
            mode.bpm()
        );
        drawn.insert(mode.bpm() as i64);
        // 鳴り始めるまでは現在のテンポを動かさない。
        assert_eq!(browser.bpm_mode(), playing_mode);
    }
    assert!(drawn.len() > 1, "テンポが引き直されていない: {drawn:?}");
}

#[test]
fn the_staged_tempo_is_adopted_only_once_the_grid_is_actually_playing() {
    let mut browser = ranged_auto_random_browser();
    let deck_path = enable_random_persistence(&mut browser, "tempo-commit");
    let playing_mode = browser.bpm_mode();

    playing(&browser, 1, 0);
    let action = browser.pump_auto_random();
    let staged = staged_mode(&action);
    let token = staged_token(&action);
    assert_eq!(browser.bpm_mode(), playing_mode);

    playing(&browser, 0, token);
    browser.pump_auto_random();
    assert_eq!(browser.bpm_mode(), staged);
    assert!(browser.track_grid_error.is_none());
    let _ = std::fs::remove_dir_all(deck_path.parent().unwrap().parent().unwrap());
}

#[test]
fn a_manual_bpm_is_never_redrawn_by_auto_random() {
    let mut browser = ranged_auto_random_browser();
    browser.set_bpm_mode(cmrt_tui_core::bpm::BpmMode::Manual(128.0));

    for cycle in (1..16).step_by(2) {
        playing(&browser, cycle, 0);
        let mode = staged_mode(&browser.pump_auto_random());
        assert_eq!(mode, cmrt_tui_core::bpm::BpmMode::Manual(128.0));
    }
}

#[test]
fn a_manual_batch_random_keeps_the_current_tempo() {
    let mut browser = ranged_auto_random_browser();
    let deck_path = enable_random_persistence(&mut browser, "manual-batch");
    let before = browser.bpm_mode();

    // `Shift+R` は明示操作なので、grid だけ引き直してテンポは据え置く。
    for _ in 0..16 {
        browser.randomize_current_measure();
        assert_eq!(browser.bpm_mode(), before);
    }
    let _ = std::fs::remove_dir_all(deck_path.parent().unwrap().parent().unwrap());
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
