//! chord ヒントと確定ダイアログ。
//!
//! ヒントの条件は「`chord2mml_core::convert` が Ok」だけ。期待値はローカルの
//! `chord2mml.exe`（`Cargo.lock` と同 revision）の Ok / Err に合わせる。
//!
//! **移送先を持たない画面では一切出ない。** 複数行モード（app の `Ctrl+P`）の
//! 挙動が 1 ビットも変わらないことを、同じ打鍵を両方へ流して確かめる。

use super::*;

use crate::state::MmlOverlayInputMode;

/// DAW と同じ開き方（1 行モード + chord 行あり）。
fn daw(initial_text: &str) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        initial_text: initial_text.to_string(),
        chord_row_transfer: true,
        ..MmlOverlayContext::default()
    });
    overlay
}

/// notepad / keyboard / grid と同じ開き方（chord 行なし）。
fn without_chord_row(initial_text: &str) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        initial_text: initial_text.to_string(),
        ..MmlOverlayContext::default()
    });
    overlay
}

fn typed(text: &str) -> MmlOverlay<'static> {
    let mut overlay = daw("");
    type_chars(&mut overlay, text, Instant::now());
    overlay
}

#[test]
fn a_chord_notation_raises_the_hint() {
    assert!(typed("I").chord_hint());
    assert!(typed("Cm7").chord_hint());
}

/// 小文字だけの MML は chord2mml が受け付けないのでヒントは立たない。
#[test]
fn plain_lowercase_mml_never_raises_the_hint() {
    assert!(!typed("cde").chord_hint());
    assert!(!typed("").chord_hint());
}

/// 打ちながら chord でなくなればヒントも下りる（立ちっぱなしにしない）。
#[test]
fn the_hint_drops_again_when_the_line_stops_parsing_as_a_chord() {
    let mut overlay = daw("");
    let now = Instant::now();
    type_chars(&mut overlay, "I", now);
    assert!(overlay.chord_hint());

    type_chars(&mut overlay, "?", now);

    assert!(!overlay.chord_hint());
}

/// 開いた時点の中身にも効く。手書きで書き込んでしまったコード表記は、
/// 開き直したときこそ気づける。
#[test]
fn the_hint_is_already_up_for_an_initial_text_that_is_a_chord() {
    assert!(daw("I-IV-V").chord_hint());
    assert!(!daw("cdefg").chord_hint());
}

/// 移送先が無い画面では、同じ文字列でもヒントは立たない。
#[test]
fn a_screen_without_a_chord_row_never_raises_the_hint() {
    let mut overlay = without_chord_row("");
    type_chars(&mut overlay, "I", Instant::now());

    assert!(!overlay.chord_hint());
}

/// 移送先が無い画面の確定は、ダイアログを挟まず従来どおり。
#[test]
fn a_screen_without_a_chord_row_commits_a_chord_notation_straight_through() {
    let mut overlay = without_chord_row("");
    let now = Instant::now();
    type_chars(&mut overlay, "I", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::Commit {
            line: "I".to_string(),
            close: false,
        }
    );
}

#[test]
fn enter_on_a_chord_notation_opens_the_confirm_dialog_instead_of_committing() {
    let mut overlay = daw("");
    let now = Instant::now();
    type_chars(&mut overlay, "I", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::Continue
    );
    assert!(overlay.chord_transfer_confirm().is_some());
    // まだ何も書いていないし、閉じてもいない。
    assert!(overlay.is_open());
    assert_eq!(overlay.value(), "I");
}

/// `Esc`（＝確定して閉じる）も同じダイアログを通る。片方だけ塞ぐと、
/// もう片方から発端のバグがそのまま通る。
#[test]
fn esc_on_a_chord_notation_also_opens_the_confirm_dialog() {
    let mut overlay = daw("");
    let now = Instant::now();
    type_chars(&mut overlay, "I", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::Continue
    );
    assert!(overlay.chord_transfer_confirm().is_some());
    assert!(overlay.is_open());
}

#[test]
fn plain_mml_commits_without_any_dialog() {
    let mut overlay = daw("");
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::Commit {
            line: "cde".to_string(),
            close: false,
        }
    );
    assert!(overlay.chord_transfer_confirm().is_none());
}

/// 既定の選択肢は移送。読まずに Enter を続けて押しても、発端のバグ
/// （無音のセル）にはならない。
#[test]
fn the_default_choice_transfers_the_line_to_the_chord_row() {
    let mut overlay = daw("");
    let now = Instant::now();
    type_chars(&mut overlay, "I-IV", now);
    overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::TransferToChordRow {
            line: "I-IV".to_string(),
        }
    );
    // カーソルが chord 行へ移るので overlay は閉じる。
    assert!(!overlay.is_open());
    assert!(overlay.chord_transfer_confirm().is_none());
}

/// 2 つ目の選択肢は、ダイアログが無かった場合と 1 ビットも変わらない確定。
#[test]
fn the_second_choice_commits_the_line_as_mml() {
    let mut overlay = daw("");
    let now = Instant::now();
    type_chars(&mut overlay, "I-IV", now);
    overlay.handle_key(press(KeyCode::Enter), now);
    overlay.handle_key(press(KeyCode::Down), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::Commit {
            line: "I-IV".to_string(),
            close: false,
        }
    );
    // `Enter` の確定なので閉じない（従来どおり）。
    assert!(overlay.is_open());
}

/// `Esc` から開いたダイアログで「MML のまま」を選ぶと、`Esc` どおり閉じる。
#[test]
fn keeping_as_mml_after_esc_closes_the_overlay_just_like_esc_would_have() {
    let mut overlay = daw("");
    let now = Instant::now();
    type_chars(&mut overlay, "I", now);
    overlay.handle_key(press(KeyCode::Esc), now);
    overlay.handle_key(press(KeyCode::Down), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::Commit {
            line: "I".to_string(),
            close: true,
        }
    );
    assert!(!overlay.is_open());
}

/// ダイアログの `Esc` は確定そのものの取り消し。入力欄へ戻るだけで何も書かない。
#[test]
fn esc_in_the_dialog_cancels_the_commit_and_returns_to_the_input() {
    let mut overlay = daw("");
    let now = Instant::now();
    type_chars(&mut overlay, "I", now);
    overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::Continue
    );
    assert!(overlay.chord_transfer_confirm().is_none());
    assert!(overlay.is_open());
    assert_eq!(overlay.value(), "I");
}

/// ダイアログが開いている間の打鍵は入力欄へ入らない（最も手前のモーダル）。
#[test]
fn typing_while_the_dialog_is_open_does_not_reach_the_input() {
    let mut overlay = daw("");
    let now = Instant::now();
    type_chars(&mut overlay, "I", now);
    overlay.handle_key(press(KeyCode::Enter), now);

    type_chars(&mut overlay, "xyz", now);

    assert_eq!(overlay.value(), "I");
    assert!(overlay.chord_transfer_confirm().is_some());
}

/// 複数行モード（app の `Ctrl+P`）は移送先を持たないので、何も変わらない。
#[test]
fn multi_line_mode_is_untouched() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "I", now);

    assert!(!overlay.chord_hint());
    // `Enter` は従来どおり改行。
    overlay.handle_key(press(KeyCode::Enter), now);
    assert_eq!(overlay.textarea().lines().len(), 2);
}
