//! Chord Chart の `x` で開く effect chain overlay の開閉・確定・試聴。

use cmrt_mml_overlay::LivePatch;
use serde_json::Value;

use super::chord_chart_auto_reverb::{
    app_with_catalogs, attach_recording_sink, playback_patch, wait_for_prepared, DEXED_SNARE,
    DRUM_ROOM_CHAIN,
};
use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn open_with_x(app: &mut TuiApp<'_>) {
    assert!(app.try_open_mml_overlay(key(KeyCode::Char('x'))));
}

fn press(app: &mut TuiApp<'_>, code: KeyCode) {
    app.handle_chord_chart_effect_chain_key_event(key(code));
}

fn app_with_snare(tag: &str) -> (cmrt_history::test_support::LocalDirGuards, TuiApp<'static>) {
    let (dirs, mut app) = app_with_catalogs(tag);
    app.chord_chart_patch = Some(DEXED_SNARE.to_string());
    (dirs, app)
}

fn chain(stages: &[Value]) -> String {
    Value::Array(stages.to_vec()).to_string()
}

#[test]
fn adding_one_stage_and_committing_puts_it_on_the_next_section_playback() {
    let (_dirs, mut app) = app_with_snare("chord_chart_effect_overlay_commit");

    open_with_x(&mut app);
    assert!(app.chord_chart_effect_overlay.is_some());
    press(&mut app, KeyCode::Char('a'));
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);

    assert!(app.chord_chart_effect_overlay.is_none());
    assert_eq!(app.chord_chart_effect_chain_stages.len(), 1);
    let stages = app.chord_chart_effect_chain_stages.clone();
    // catalog の preset は reverb だけなので、手動 reverb が auto reverb を置き換える。
    assert_eq!(
        playback_patch(&app, 0),
        LivePatch::with_effect_chain(Some(DEXED_SNARE), &chain(&stages))
    );
    assert_eq!(
        crate::history::load_session_state()
            .unwrap()
            .chord_chart_effect_chain,
        stages
    );
}

#[test]
fn esc_discards_the_edited_chain() {
    let (_dirs, mut app) = app_with_snare("chord_chart_effect_overlay_esc");

    open_with_x(&mut app);
    press(&mut app, KeyCode::Char('a'));
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Esc);

    assert!(app.chord_chart_effect_overlay.is_none());
    assert!(app.chord_chart_effect_chain_stages.is_empty());
    assert_eq!(
        playback_patch(&app, 0),
        LivePatch::with_effect_chain(Some(DEXED_SNARE), DRUM_ROOM_CHAIN)
    );
}

#[test]
fn without_an_effect_catalog_it_does_not_open_and_says_why() {
    let mut app = super::chord_chart_preview::app_on_the_chord_chart();

    open_with_x(&mut app);

    assert!(app.chord_chart_effect_overlay.is_none());
    assert_eq!(
        app.chord_chart.error.as_deref(),
        Some(cmrt_effect_chain_select::messages::NOT_AVAILABLE_ON_THIS_BACKEND)
    );
}

/// 開いた直後は確定済みの chain で音色を準備し、`a` の候補はその候補を載せて section を鳴らす。
///
/// `a` 直後の候補は auto reverb と同じ preset なので、`j` で別の候補へ移ってから見る
/// （同じ音色と chain の準備は sender が省くため、件数で区別できない）。
#[test]
fn opening_and_moving_to_a_candidate_audition_the_chord_with_that_chain() {
    let (_dirs, mut app) = app_with_snare("chord_chart_effect_overlay_audition");
    let sink = attach_recording_sink(&mut app);

    open_with_x(&mut app);
    assert_eq!(
        wait_for_prepared(&sink, 1)[0],
        LivePatch::with_effect_chain(Some(DEXED_SNARE), DRUM_ROOM_CHAIN)
    );

    press(&mut app, KeyCode::Char('a'));
    press(&mut app, KeyCode::Char('j'));
    let candidate = app
        .chord_chart_effect_overlay
        .as_ref()
        .and_then(|overlay| {
            let cursor = overlay.editor.add.list_cursor;
            overlay
                .editor
                .candidate_chain(app.effect_plugins.catalog(), cursor, false)
        })
        .expect("candidate");
    assert_eq!(
        wait_for_prepared(&sink, 2)[1],
        LivePatch::with_effect_chain(Some(DEXED_SNARE), &chain(&candidate))
    );
    assert!(
        wait_for_timelines(&sink, 1) >= 1,
        "候補の試聴で section が鳴っていない"
    );
}

fn wait_for_timelines(sink: &cmrt_mml_overlay::RecordingSink, count: usize) -> usize {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while sink.timelines() < count && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    sink.timelines()
}

#[test]
fn x_is_not_taken_while_the_help_is_open() {
    let (_dirs, mut app) = app_with_snare("chord_chart_effect_overlay_help");
    app.chord_chart.help_open = true;

    assert!(!app.try_open_mml_overlay(key(KeyCode::Char('x'))));
    assert!(app.chord_chart_effect_overlay.is_none());
}

#[test]
fn the_overlay_is_drawn_over_the_chord_chart_and_names_the_chord_patch() {
    let (_dirs, mut app) = app_with_snare("chord_chart_effect_overlay_draw");

    open_with_x(&mut app);

    // 全角文字の後ろの cell は空白で読めるので、空白を除いて照合する。
    let screen = crate::tui::ui::tests::render_lines(&mut app, 100, 30)
        .join("\n")
        .replace(' ', "");
    assert!(screen.contains("EFFECTCHAIN"), "{screen}");
    let header = format!("Chord音色:{DEXED_SNARE}").replace(' ', "");
    assert!(screen.contains(&header), "{screen}");
}
