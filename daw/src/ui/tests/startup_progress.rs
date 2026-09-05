//! 「音が鳴るまで」overlay が DAW の画面に実際に出るか。
//!
//! 段階の翻訳規則は `ui/startup_progress/tests.rs` が持つ。ここが見るのは
//! **画面へ描かれるか／消えるか**だけ。

use super::*;

/// 全角文字は TestBackend 上で「文字＋空セル」に割れるので、空白を落として比べる
/// （`find_text_ignoring_spaces` と同じ規則）。
fn screen(app: &DawApp) -> String {
    render_lines(app, 160, 40).join("").replace(' ', "")
}

#[test]
fn nothing_is_drawn_while_no_play_is_starting() {
    let app = build_test_app();

    assert!(!screen(&app).contains("音が鳴るまで"));
}

#[test]
fn the_overlay_shows_the_play_server_stage_while_waiting_for_it() {
    let app = build_test_app();
    app.playback.startup.begin(true);

    let rendered = screen(&app);

    assert!(rendered.contains("音が鳴るまで"), "{rendered}");
    assert!(rendered.contains("playserver起動"), "{rendered}");
    assert!(rendered.contains("1小節目の音色ロード"), "{rendered}");
}

#[test]
fn the_overlay_counts_the_tracks_of_the_first_measure() {
    let app = build_test_app();
    app.playback.startup.begin(true);
    app.playback.startup.begin_first_measure(7);
    app.playback.startup.note_measure_loaded(3);

    let rendered = screen(&app);

    assert!(rendered.contains("3/7"), "{rendered}");
}

/// 音が出たら消える。出しっぱなしにするとグリッドが読めない。
#[test]
fn the_overlay_disappears_once_the_wait_is_over() {
    let app = build_test_app();
    app.playback.startup.begin(true);
    app.playback.startup.finish();

    assert!(!screen(&app).contains("音が鳴るまで"));
}
