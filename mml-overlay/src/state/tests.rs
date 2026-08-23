mod history;
mod line;
mod patch;

use std::time::Duration;

use super::*;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn opened() -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext::default());
    overlay
}

/// 文字を順に打ち込む。最後の 1 打鍵ぶんのアクションを返す。
fn type_chars(overlay: &mut MmlOverlay<'_>, text: &str, now: Instant) -> MmlOverlayAction {
    let mut action = MmlOverlayAction::Continue;
    for code in text.chars().map(KeyCode::Char) {
        action = overlay.handle_key(press(code), now);
    }
    action
}

fn send(messages: Vec<[u8; 3]>, duration_ms: u64) -> MmlOverlayAction {
    MmlOverlayAction::Send(NoteRequest {
        messages,
        duration: Duration::from_millis(duration_ms),
    })
}

#[test]
fn typing_a_note_sends_note_on() {
    let mut overlay = opened();
    let now = Instant::now();

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('c')), now),
        send(vec![[0x90, 60, 127]], 250)
    );
    assert_eq!(overlay.sounding(), [60]);
}

/// 次の音は note on だけ返す。前の音を止めるのは、受け取った側が
/// 「鳴らす前に必ず止める」で行う。ここで note off を組み立てないのが要点。
#[test]
fn typing_the_next_note_asks_for_the_new_note_only() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('d')), now),
        send(vec![[0x90, 62, 127]], 250)
    );
}

#[test]
fn moving_the_cursor_left_sounds_the_earlier_note_again() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cd", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Left), now),
        send(vec![[0x90, 60, 127]], 250)
    );
}

/// 和音は閉じ `'` まで含めて 1 つの発音単位。中をカーソルが通っても鳴らし直さない。
#[test]
fn moving_inside_a_chord_does_not_resound_it() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "'ceg'", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Left), now),
        MmlOverlayAction::Continue
    );
    assert_eq!(
        overlay.handle_key(press(KeyCode::Left), now),
        MmlOverlayAction::Continue
    );
}

/// 別の発音単位へ移れば鳴る。単音から和音へ戻るところ。
#[test]
fn moving_from_a_note_back_into_a_chord_sounds_the_chord() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "'ceg'c", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Left), now),
        send(vec![[0x90, 60, 127], [0x90, 64, 127], [0x90, 67, 127]], 250,)
    );
}

/// カーソルが動かなければ鳴らさない。行末で → を押しても無音のまま。
#[test]
fn pressing_right_at_the_end_of_the_line_stays_silent() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "c", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Right), now),
        MmlOverlayAction::Continue
    );
}

/// 休符を打った瞬間は鳴らし直さない。カーソル移動だけを特別扱いする。
#[test]
fn typing_a_rest_does_not_resound_the_previous_note() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('r')), now),
        MmlOverlayAction::Continue
    );
}

#[test]
fn a_modifier_resounds_the_same_note_shifted() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('+')), now),
        send(vec![[0x90, 61, 127]], 250)
    );
}

/// コマンドは発音単位ではないので、その上では鳴らさない。鳴っている音は
/// gate が切るまでそのまま。
#[test]
fn a_command_that_adds_no_note_stays_silent() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('>')), now),
        MmlOverlayAction::Continue
    );
    assert_eq!(overlay.sounding(), [60]);
}

#[test]
fn a_chord_sounds_every_member_at_once() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "'ce", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('g')), now),
        send(vec![[0x90, 60, 127], [0x90, 64, 127], [0x90, 67, 127]], 250,)
    );
}

#[test]
fn a_bare_note_requests_its_full_mml_duration() {
    let mut overlay = opened();
    let now = Instant::now();
    let action = overlay.handle_key(press(KeyCode::Char('c')), now);

    // 既定の 8 分音符を既定テンポ 120 で鳴らした長さをsenderへそのまま渡す。
    assert_eq!(action, send(vec![[0x90, 60, 127]], 250));
}

/// 音長を書き足したら、その長さで鳴らし直す。`c` のあと `1` を打っても
/// 既定の 8 分音符のままだと、書いた音長を耳で確かめられない。
#[test]
fn writing_a_note_length_resounds_the_note_for_that_length() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('1')), now),
        send(vec![[0x90, 60, 127]], 2000)
    );
    // 全音符 = 既定テンポ 120 で 2 秒。senderは実発音開始後からこの長さを数える。
}

/// テンポ指定も音長へ効く。`t60` なら 8 分音符は 500ms。
#[test]
fn a_tempo_command_stretches_the_gate() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "t60", now);
    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('c')), now),
        send(vec![[0x90, 60, 127]], 500)
    );
}

#[test]
fn closing_stops_every_sounding_note() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::Close
    );
    assert!(!overlay.is_open());
    assert!(overlay.sounding().is_empty());
}

#[test]
fn reopening_starts_from_an_empty_input() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(press(KeyCode::Enter), now);
    overlay.handle_key(press(KeyCode::Esc), now);

    overlay.open(MmlOverlayContext::default());
    assert_eq!(overlay.value(), "");
    assert!(overlay.is_open());
}

#[test]
fn deleting_the_last_note_and_typing_it_again_sounds_it() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);
    overlay.handle_key(press(KeyCode::Backspace), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('c')), now),
        send(vec![[0x90, 60, 127]], 250)
    );
}

#[test]
fn sender_status_tracks_actual_sound_and_ignores_an_older_command() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('C')), now);
    overlay.expect_sender_command(2);

    overlay.sync_sender_status(&MmlOverlaySenderStatus {
        command_id: 1,
        sounding: Vec::new(),
        ..MmlOverlaySenderStatus::default()
    });
    assert_eq!(overlay.sounding(), [60, 64, 67]);

    overlay.sync_sender_status(&MmlOverlaySenderStatus {
        command_id: 2,
        loading: true,
        sounding: Vec::new(),
        ..MmlOverlaySenderStatus::default()
    });
    assert!(overlay.sounding().is_empty());

    overlay.sync_sender_status(&MmlOverlaySenderStatus {
        command_id: 2,
        sounding: vec![60, 64, 67],
        ..MmlOverlaySenderStatus::default()
    });
    assert_eq!(overlay.sounding(), [60, 64, 67]);
    assert!(overlay.sounding_from_chord());

    overlay.sync_sender_status(&MmlOverlaySenderStatus {
        command_id: 2,
        sounding: Vec::new(),
        ..MmlOverlaySenderStatus::default()
    });
    assert!(overlay.sounding().is_empty());
}
