use ratatui::{backend::TestBackend, Terminal};

use super::*;

fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_overlay(phase: KeyboardConnectionPhase) -> String {
    render_overlay_with_progress(phase, None)
}

fn render_overlay_with_progress(
    phase: KeyboardConnectionPhase,
    server_startup: Option<(usize, usize)>,
) -> String {
    let connection = KeyboardConnectionStatus {
        phase,
        server_startup,
        stage_started_at: Some(std::time::Instant::now()),
        ..KeyboardConnectionStatus::default()
    };
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| draw_connection_overlay(&connection, f, f.area()))
        .unwrap();
    buffer_to_string(&terminal)
}

/// 起動待ちは「あと何本の CLAP instance か」まで出す。以前は
/// 「connecting...」の 1 行だけで、進んでいるのか固まったのかが分からなかった。
#[test]
fn connecting_overlay_counts_the_clap_instances_of_the_play_server() {
    let screen = render_overlay_with_progress(KeyboardConnectionPhase::Connecting, Some((9, 14)));
    let normalized = screen.replace(' ', "");

    assert!(normalized.contains("音が鳴るまで"), "{screen}");
    assert!(normalized.contains("playserver起動"), "{screen}");
    assert!(normalized.contains("9/14"), "{screen}");
    assert!(normalized.contains("経過"), "{screen}");
}

#[test]
fn error_overlay_shows_retry_navigation() {
    let screen = render_overlay(KeyboardConnectionPhase::Error("server failed".to_string()));

    assert!(screen.contains("server error: server failed"));
    assert!(screen.contains("r:retry"));
}

#[test]
fn patch_setting_overlay_remains_until_patch_is_ready() {
    let screen = render_overlay(KeyboardConnectionPhase::PatchSetting);
    let normalized = screen.replace(' ', "");

    // server の段は済み、音色ロードの段が動いている。
    assert!(normalized.contains("✓playserver起動"), "{screen}");
    assert!(normalized.contains("▶音色ロード"), "{screen}");
}

/// 鳴らせる状態になったら overlay は消える。
#[test]
fn no_overlay_is_drawn_once_the_keyboard_is_ready() {
    assert!(render_overlay(KeyboardConnectionPhase::Ready)
        .chars()
        .all(char::is_whitespace));
}

/// まだ何も要求していない状態は、段階を並べても全部 Waiting になるだけなので
/// 「鳴らない理由」を 1 行で出す従来のままにしてある。
#[test]
fn idle_keeps_the_plain_notice() {
    let screen = render_overlay(KeyboardConnectionPhase::Idle);

    assert!(screen.contains("connecting..."), "{screen}");
    assert!(screen.contains("notes unavailable until ready"), "{screen}");
}
