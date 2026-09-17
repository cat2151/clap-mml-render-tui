use std::time::Duration;

use super::*;
use crate::{step_offset, GridScheduledMessage, CHORD_ROW};

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;

/// chord 行の 3 音を止めてから鳴らし直す列。bank 0 なので宛先はどちらも instance 0。
const CHORD_REATTACK: [(u8, u8, u8); 6] = [
    (NOTE_OFF, 0, 60),
    (NOTE_OFF, 0, 64),
    (NOTE_OFF, 0, 67),
    (NOTE_ON, 0, 60),
    (NOTE_ON, 0, 64),
    (NOTE_ON, 0, 67),
];

/// chord mode ON・chord 行が Keys/Alpha で、小節の途中（step 5）まで鳴らした画面。
fn sounding_chord_screen() -> GridSequencerScreen {
    let now = Instant::now();
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.rows_mut()[CHORD_ROW].patch = Some("Keys/Alpha.fxp".to_string());
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        now,
    );
    screen.state.start(now);
    for step in 0..6 {
        screen
            .state
            .poll_steps(now + step_offset(step), Duration::ZERO);
    }
    screen
}

/// 記録した送信を取り出して空にする。
fn take_sent(screen: &GridSequencerScreen) -> Vec<(u8, u8, u8)> {
    std::mem::take(&mut *screen.sent.borrow_mut())
        .iter()
        .map(|scheduled: &GridScheduledMessage| {
            (
                scheduled.message[0],
                scheduled.instance_id,
                scheduled.message[1],
            )
        })
        .collect()
}

/// `j` で試聴した瞬間に、鳴っていた和音が鳴り直す。同じ patch を続けて試聴しても
/// ロードが起きないので鳴らし直さない。
#[test]
fn a_preview_restarts_the_sounding_chord_once_per_loaded_patch() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = sounding_chord_screen();
    screen.open_patch_selector(CHORD_ROW, &ctx);
    take_sent(&screen);

    screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    assert_eq!(
        selector(&screen).previewed_patch.as_deref(),
        Some("Keys/Beta.fxp")
    );
    assert_eq!(take_sent(&screen), CHORD_REATTACK);

    // 末尾で End を重ねても選択は変わらないので、ロードも鳴らし直しも無い。
    screen.handle_patch_selector_key(press(KeyCode::End), &ctx);
    take_sent(&screen);
    screen.handle_patch_selector_key(press(KeyCode::End), &ctx);
    assert!(take_sent(&screen).is_empty());
}

#[test]
fn confirming_a_previewed_patch_restarts_the_sounding_chord() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = sounding_chord_screen();
    screen.open_patch_selector(CHORD_ROW, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    take_sent(&screen);

    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);

    assert!(screen.patch_selector.is_none());
    assert_eq!(
        screen.state.rows()[CHORD_ROW].patch.as_deref(),
        Some("Keys/Beta.fxp")
    );
    assert_eq!(take_sent(&screen), CHORD_REATTACK);
}

#[test]
fn cancelling_after_a_preview_restarts_the_sounding_chord_with_the_original_patch() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = sounding_chord_screen();
    screen.open_patch_selector(CHORD_ROW, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    take_sent(&screen);

    screen.handle_patch_selector_key(press(KeyCode::Esc), &ctx);

    assert!(screen.patch_selector.is_none());
    assert_eq!(
        screen.state.rows()[CHORD_ROW].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );
    assert_eq!(take_sent(&screen), CHORD_REATTACK);
}

/// undo で音色が戻るときも、そのロードで消える和音を鳴らし直す。
#[test]
fn undoing_a_patch_change_restarts_the_sounding_chord() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = sounding_chord_screen();
    screen.open_patch_selector(CHORD_ROW, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Enter), &ctx);
    take_sent(&screen);

    screen.undo(&ctx);

    assert_eq!(
        screen.state.rows()[CHORD_ROW].patch.as_deref(),
        Some("Keys/Alpha.fxp")
    );
    assert_eq!(take_sent(&screen), CHORD_REATTACK);
}

/// 停止中はロードだけで、鳴らし直す音は無い。
#[test]
fn a_preview_while_stopped_sends_nothing() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.rows_mut()[CHORD_ROW].patch = Some("Keys/Alpha.fxp".to_string());
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen.open_patch_selector(CHORD_ROW, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);

    assert_eq!(
        selector(&screen).previewed_patch.as_deref(),
        Some("Keys/Beta.fxp")
    );
    assert!(take_sent(&screen).is_empty());
}
