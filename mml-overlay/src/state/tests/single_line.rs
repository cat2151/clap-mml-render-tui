//! 1 行モード（`MmlOverlayInputMode::SingleLine`）。
//!
//! 「1 か所へ書き戻すための入力欄」なので `Enter` は改行ではなく確定になる。
//! 複数行モードの挙動は 1 ビットも変わってはいけないので、同じ打鍵を
//! 両モードへ流して結果が食い違うことを毎回 assert する。

use super::*;

use crate::state::MmlOverlayInputMode;

fn single_line(initial_text: &str) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        initial_text: initial_text.to_string(),
        ..MmlOverlayContext::default()
    });
    overlay
}

fn commit(line: &str, close: bool) -> MmlOverlayAction {
    MmlOverlayAction::Commit {
        line: line.to_string(),
        close,
    }
}

#[test]
fn enter_commits_the_line_instead_of_inserting_a_newline() {
    let mut overlay = single_line("");
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        commit("cde", false)
    );
    // 改行が入っていない（入力欄は 1 行のまま）。
    assert_eq!(overlay.textarea().lines().len(), 1);
    assert_eq!(overlay.value(), "cde");
    // 確定しても閉じない。次に何を編集するかはホストが決める。
    assert!(overlay.is_open());
}

/// 端末によっては `Enter` が `Ctrl+M` として届く。どちらでも確定になること。
#[test]
fn ctrl_m_commits_exactly_like_enter() {
    let mut overlay = single_line("");
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);

    assert_eq!(
        overlay.handle_key(ctrl(KeyCode::Char('m')), now),
        commit("cde", false)
    );
    assert_eq!(overlay.textarea().lines().len(), 1);
    assert_eq!(overlay.value(), "cde");
}

#[test]
fn esc_commits_and_closes() {
    let mut overlay = single_line("");
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        commit("cde", true)
    );
    assert!(!overlay.is_open());
    assert!(overlay.sounding().is_empty());
}

#[test]
fn opens_with_the_initial_text_and_the_cursor_at_its_end() {
    let overlay = single_line("cdefg");

    assert_eq!(overlay.value(), "cdefg");
    let DataCursor(row, column) = overlay.textarea().cursor();
    assert_eq!((row, column), (0, 5));
}

/// ホストが複数行を渡しても入力欄は 1 行。改行を持ち込むと `Enter` の意味が壊れる。
#[test]
fn a_multiline_initial_text_keeps_only_its_first_line() {
    let overlay = single_line("cde\nfga");

    assert_eq!(overlay.textarea().lines().len(), 1);
    assert_eq!(overlay.value(), "cde");
}

#[test]
fn an_empty_input_commits_an_empty_line() {
    let mut overlay = single_line("cde");
    let now = Instant::now();
    for _ in 0..3 {
        overlay.handle_key(press(KeyCode::Backspace), now);
    }

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        commit("", false)
    );
}

/// 打鍵の発音は 1 行モードでも従来どおり。入力欄の行数だけが違う。
#[test]
fn typing_a_note_still_sends_note_on() {
    let mut overlay = single_line("");
    let now = Instant::now();

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('c')), now),
        send(vec![[0x90, 60, 127]], 250)
    );
}

#[test]
fn ctrl_space_still_replays_the_line() {
    let mut overlay = single_line("cde");
    let now = Instant::now();

    assert!(matches!(
        overlay.handle_key(ctrl(KeyCode::Char(' ')), now),
        MmlOverlayAction::PlayLine { .. }
    ));
}

/// `Ctrl+T` の一覧の `Enter` は候補の確定。入力欄の確定に横取りされてはいけない。
#[test]
fn enter_inside_the_patch_select_confirms_the_patch_and_does_not_commit() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        initial_text: "cde".to_string(),
        patch_catalog: PatchCatalogSnapshot::Ready(
            ["Leads/Lead 1.fxp", "Pads/Pad 1.fxp"]
                .into_iter()
                .map(|patch| PatchCatalogEntry::from_display(patch.to_string()))
                .collect(),
        ),
        ..MmlOverlayContext::default()
    });
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::Continue
    );
    assert_eq!(overlay.patch(), Some("Leads/Lead 1.fxp"));
    assert_eq!(overlay.value(), "cde");
    assert!(overlay.is_open());
}

/// `Ctrl+L` の演奏設定の `Enter` も同じく横取りされない。
#[test]
fn enter_inside_the_play_settings_confirms_them_and_does_not_commit() {
    let mut overlay = single_line("cde");
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::Continue
    );
    assert!(overlay.play_settings().repeat);
    assert!(overlay.is_open());
}

/// `Ctrl+O` で確定した履歴は 1 行として入る。複数行モードの入力欄へ戻すと
/// `Enter` が改行に戻ってしまう。
#[test]
fn a_confirmed_history_entry_stays_a_single_line() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        history: vec!["gfedc".to_string()],
        ..MmlOverlayContext::default()
    });
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(overlay.textarea().lines().len(), 1);
    assert_eq!(overlay.value(), "gfedc");
    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        commit("gfedc", false)
    );
}

// --- 複数行モードが変わっていないこと ---

#[test]
fn multi_line_enter_still_inserts_a_newline_and_never_commits() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);

    let action = overlay.handle_key(press(KeyCode::Enter), now);
    assert!(
        !matches!(action, MmlOverlayAction::Commit { .. }),
        "複数行モードで Commit が返ってはいけない: {action:?}"
    );
    assert_eq!(overlay.textarea().lines().len(), 2);
    assert_eq!(overlay.value(), "cde\n");
}

#[test]
fn multi_line_ctrl_m_still_inserts_a_newline_and_never_commits() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);

    let action = overlay.handle_key(ctrl(KeyCode::Char('m')), now);
    assert!(
        !matches!(action, MmlOverlayAction::Commit { .. }),
        "複数行モードで Commit が返ってはいけない: {action:?}"
    );
    assert_eq!(overlay.textarea().lines().len(), 2);
}

#[test]
fn multi_line_esc_still_closes_without_committing() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::Close
    );
    assert!(!overlay.is_open());
}

/// 既定は複数行。ホストが何も言わなければ従来の入力欄で開く。
#[test]
fn the_default_context_opens_in_multi_line_mode() {
    let overlay = opened();

    assert_eq!(overlay.input_mode(), MmlOverlayInputMode::MultiLine);
}

/// 複数行モードは初期テキストを受け取らない（従来どおり必ず空で開く）。
#[test]
fn multi_line_ignores_the_initial_text() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        initial_text: "cde".to_string(),
        ..MmlOverlayContext::default()
    });

    assert_eq!(overlay.value(), "");
}

/// 開き直しでモードは切り替わる。DAW が 1 行で開いたあと、app が複数行で開く経路。
#[test]
fn reopening_switches_the_input_mode() {
    let mut overlay = single_line("cde");
    assert_eq!(overlay.input_mode(), MmlOverlayInputMode::SingleLine);

    overlay.open(MmlOverlayContext::default());

    assert_eq!(overlay.input_mode(), MmlOverlayInputMode::MultiLine);
    assert_eq!(overlay.value(), "");
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);
    assert!(!matches!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::Commit { .. }
    ));
}
