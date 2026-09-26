//! 音色 selector の候補を動かすと、前の候補を fadeout してから次を鳴らすことの検証。

use cmrt_mml_overlay::SinkOperation;
use crossterm::event::KeyCode;

use super::super::direct_patch_select::tests::{
    app_with_generated_empty_cell, attach_recording_sink, key, plain, wait_until,
};

#[test]
fn moving_to_the_next_candidate_fades_out_the_previous_line_first() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_generated_empty_cell();
    let sink = attach_recording_sink(&mut app);
    app.handle_normal_key_event(plain('t'));
    wait_until("開いた時点の試聴", || sink.timelines() >= 1);

    // 一覧は Bass, Pad の順でカーソルは Pad にあるので、上へ動かす。
    app.handle_mml_overlay_key_event(key(KeyCode::Up));
    wait_until("次の候補の試聴", || sink.timelines() >= 2);

    let operations = sink.operations();
    let fade = operations
        .iter()
        .position(|operation| matches!(operation, SinkOperation::FadeOut { .. }))
        .expect("前の候補を fadeout すること");
    let second_line = operations
        .iter()
        .enumerate()
        .filter(|(_, operation)| **operation == SinkOperation::Timeline)
        .nth(1)
        .map(|(index, _)| index)
        .unwrap();
    assert!(fade < second_line, "{operations:?}");
}
