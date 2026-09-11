//! 「音が鳴るまで」の overlay が、実際に描かれた画面へ出る／出ないことの確認。
//!
//! chord chart の preview は押してから最初の音まで数秒掛かるのに、その間**画面には何も出ていなかった**。直したかったのはそこなので、
//! ここは状態ではなく描画された buffer を読む。

use std::time::{Duration, Instant};

use super::{render_lines, test_config};
use crate::tui::sound_startup_overlay::SoundStartupWait;
use crate::tui::TuiApp;

/// 全角は buffer 上でセル 2 つぶんなので、空白を落としてから照合する。
fn contains_ignoring_spaces(lines: &[String], text: &str) -> bool {
    let strip = |value: &str| -> String { value.chars().filter(|c| !c.is_whitespace()).collect() };
    let needle = strip(text);
    lines.iter().any(|line| strip(line).contains(&needle))
}

fn chord_chart_app_waiting_for_sound(server_startup: Option<(usize, usize)>) -> TuiApp<'static> {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::ChordChart;
    app.sound_startup_wait = Some(SoundStartupWait {
        started_at: Instant::now() - Duration::from_secs(3),
        server_startup,
    });
    app
}

/// 出る条件: 待っているあいだは段階と経過秒数が中央に出る。
#[test]
fn the_chord_chart_shows_the_startup_progress_while_waiting_for_sound() {
    let mut app = chord_chart_app_waiting_for_sound(Some((5, 14)));

    let lines = render_lines(&mut app, 120, 30);

    assert!(
        contains_ignoring_spaces(&lines, "音が鳴るまで"),
        "待っていることを中央に出すこと: {lines:?}"
    );
    assert!(
        contains_ignoring_spaces(&lines, "play server 起動"),
        "いま何を待っているのかを出すこと: {lines:?}"
    );
    assert!(
        contains_ignoring_spaces(&lines, "音源の準備"),
        "次に何が残っているのかを出すこと: {lines:?}"
    );
    assert!(
        contains_ignoring_spaces(&lines, "5/14"),
        "進み具合（実測できる instance の本数）を出すこと: {lines:?}"
    );
    assert!(
        contains_ignoring_spaces(&lines, "経過3."),
        "何秒待っているのかを出すこと: {lines:?}"
    );
}

/// 消える条件: 待ちが無くなったら消える。
#[test]
fn the_overlay_disappears_once_the_wait_is_over() {
    let mut app = chord_chart_app_waiting_for_sound(Some((14, 14)));
    app.sound_startup_wait = None;

    let lines = render_lines(&mut app, 120, 30);

    assert!(
        !contains_ignoring_spaces(&lines, "音が鳴るまで"),
        "鳴り始めたら消すこと: {lines:?}"
    );
}

/// MML オーバーレイが開いているあいだは出さない（あちらが自前の loading 表示を持つ）。
#[test]
fn the_overlay_stays_out_of_the_mml_overlay() {
    let mut app = chord_chart_app_waiting_for_sound(Some((5, 14)));
    app.mml_overlay
        .open(cmrt_mml_overlay::MmlOverlayContext::default());

    let lines = render_lines(&mut app, 120, 30);

    assert!(
        !contains_ignoring_spaces(&lines, "音が鳴るまで"),
        "MML オーバーレイの loading 表示と二重に出さないこと: {lines:?}"
    );
}
