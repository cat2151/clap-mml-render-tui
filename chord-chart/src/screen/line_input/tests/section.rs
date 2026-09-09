//! `i` / `n` の 1 行入力を、実際のキーの並びから確かめる。

use super::*;

#[test]
fn i_opens_the_input_prefilled_with_the_current_progression() {
    let mut screen = two_section_screen();

    assert_eq!(
        screen.handle_key_event(key('i')),
        ChordChartAction::Continue
    );

    assert!(screen.line_input_open());
    assert_eq!(
        screen.line_input().unwrap().target(),
        LineInputTarget::Degrees
    );
    // 既定値が入っていないと、少し直したいだけでも全部打ち直しになる。
    assert_eq!(input_value(&screen), "I-V-VIm-IV");
}

#[test]
fn n_opens_the_input_prefilled_with_the_current_name() {
    let mut screen = two_section_screen();
    screen.section_cursor = 1;

    screen.handle_key_event(key('n'));

    assert_eq!(screen.line_input().unwrap().target(), LineInputTarget::Name);
    assert_eq!(input_value(&screen), "B");
}

/// 進行を打ち直すと、その section の degrees だけが変わる。
#[test]
fn a_committed_progression_changes_only_the_section_under_the_cursor() {
    let mut screen = two_section_screen();
    assert_eq!(degrees_of(&screen, 0), "I-V-VIm-IV");

    screen.handle_key_event(key('i'));
    clear_input(&mut screen);
    type_text(&mut screen, "I-IV-V-I-VIm-IV");

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::SongChanged
    );
    assert!(!screen.line_input_open());
    assert_eq!(degrees_of(&screen, 0), "I-IV-V-I-VIm-IV");
    // 編集していない section は触らない。
    assert_eq!(degrees_of(&screen, 1), "IIm-V-I-VIm");
}

/// 端末によっては `Enter` が `Ctrl+M` として届く。両方で確定できること。
#[test]
fn control_m_commits_just_like_enter() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    type_text(&mut screen, "Sabi");

    assert_eq!(
        screen.handle_key_event(ctrl('m')),
        ChordChartAction::SongChanged
    );
    assert!(!screen.line_input_open());
    assert_eq!(screen.song.sections[0].name, "Sabi");
}

/// この画面は degrees を解釈しない。**読めない進行もそのまま確定する**
/// （書式は chord2mml-rs のもので、読めるかどうかを決めるのは演奏側＝別スコープ）。
#[test]
fn a_progression_this_screen_cannot_read_is_committed_all_the_same() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('i'));
    clear_input(&mut screen);
    type_text(&mut screen, "zzz");

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::SongChanged
    );

    assert!(!screen.line_input_open());
    assert_eq!(degrees_of(&screen, 0), "zzz");
    // 理由は 1 つも出ない（下段にも overlay にも）。
    assert_eq!(screen.error, None);
}

/// 空の degrees も受ける（`b` の prefix と同じ扱い）。空を弾く相手は名前だけ。
#[test]
fn an_empty_progression_is_accepted_too() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('i'));
    clear_input(&mut screen);

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::SongChanged
    );

    assert!(!screen.line_input_open());
    assert_eq!(degrees_of(&screen, 0), "");
}

/// 理由が出たあと文字を直したら、その場で理由は消える（古い理由が残らない）。
/// 理由が出るのは名前が空のときだけになった。
#[test]
fn typing_after_a_rejected_commit_clears_the_reason() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    screen.handle_key_event(plain(KeyCode::Enter));
    assert!(screen.line_input().unwrap().error().is_some());

    type_text(&mut screen, "S");

    assert!(screen.line_input().unwrap().error().is_none());
}

/// 直してから確定し直せる＝入力欄が「直す場所」として機能している。
#[test]
fn a_rejected_name_can_be_fixed_without_reopening_the_input() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    screen.handle_key_event(plain(KeyCode::Enter));

    type_text(&mut screen, "Sabi");

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::SongChanged
    );
    assert!(!screen.line_input_open());
    assert_eq!(screen.song.sections[0].name, "Sabi");
}

/// 空の名前では確定させない（左 pane の行が読めなくなる）。
#[test]
fn an_empty_name_keeps_the_input_open() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::Continue
    );

    assert!(screen.line_input_open());
    assert_eq!(
        screen.line_input().unwrap().error(),
        Some(EMPTY_NAME_MESSAGE)
    );
    assert_eq!(screen.song.sections[0].name, "A");
}

/// 空白だけの名前も空と同じ（見た目は空行になる）。
#[test]
fn a_whitespace_only_name_counts_as_empty() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    type_text(&mut screen, "   ");

    screen.handle_key_event(plain(KeyCode::Enter));

    assert!(screen.line_input_open());
    assert_eq!(screen.song.sections[0].name, "A");
}

/// 前後の空白は落として確定する。
#[test]
fn the_committed_value_is_trimmed() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('n'));
    clear_input(&mut screen);
    type_text(&mut screen, "  Sabi  ");
    screen.handle_key_event(plain(KeyCode::Enter));

    assert_eq!(screen.song.sections[0].name, "Sabi");
}

#[test]
fn escape_closes_the_input_and_keeps_the_old_value() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('i'));
    clear_input(&mut screen);
    type_text(&mut screen, "I-IV");

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Esc)),
        ChordChartAction::Continue
    );

    assert!(!screen.line_input_open());
    assert_eq!(degrees_of(&screen, 0), "I-V-VIm-IV");
}

/// 値が変わらない確定でファイルを書き直さない（`Continue` を返す）。
#[test]
fn committing_the_same_value_does_not_ask_for_a_save() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('i'));

    assert_eq!(
        screen.handle_key_event(plain(KeyCode::Enter)),
        ChordChartAction::Continue
    );
    assert!(!screen.line_input_open());
    assert_eq!(degrees_of(&screen, 0), "I-V-VIm-IV");
}

/// 開いている間は他のキーが裏の画面に届かない。`?` も `Tab` も文字として入る。
#[test]
fn the_input_swallows_every_other_key() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('i'));
    clear_input(&mut screen);

    screen.handle_key_event(key('?'));
    screen.handle_key_event(plain(KeyCode::Tab));
    screen.handle_key_event(key('j'));

    assert!(!screen.help_open);
    assert_eq!(screen.focus, Pane::Sections);
    assert_eq!(screen.section_cursor, 0);
    assert!(screen.line_input_open());
}

/// `Ctrl+W`（単語削除）が効く＝自前の push/pop ではなく textarea を通している。
/// ここが落ちたら「1 行入力なら当然効くべき keybind」が失われている。
#[test]
fn the_textarea_keybinds_such_as_control_w_are_available() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('i'));
    clear_input(&mut screen);
    type_text(&mut screen, "I-V VIm");

    screen.handle_key_event(ctrl('w'));

    assert_eq!(input_value(&screen), "I-V ");
}

/// section が 1 つも無いときは開かない（書き込む先が無い）。
#[test]
fn e_and_n_do_nothing_without_a_section() {
    let mut screen = ChordChartScreen::new(Song::empty());

    for code in ['e', 'n'] {
        assert_eq!(
            screen.handle_key_event(key(code)),
            ChordChartAction::Continue
        );
        assert!(!screen.line_input_open());
    }
}

/// Arrangement pane では `e` / `n` が効かない（左 pane 専用のキー）。
#[test]
fn e_and_n_are_inert_while_the_arrangement_pane_has_focus() {
    let mut screen = two_section_screen();
    screen.focus = Pane::Arrangement;

    for code in ['e', 'n'] {
        screen.handle_key_event(key(code));
        assert!(!screen.line_input_open());
    }
}

/// カーソルを動かしてから開くと、その section が対象になる。
#[test]
fn the_input_edits_the_section_under_the_cursor() {
    let mut screen = two_section_screen();
    screen.handle_key_event(key('j'));

    screen.handle_key_event(key('i'));
    clear_input(&mut screen);
    type_text(&mut screen, "I-IV");
    screen.handle_key_event(plain(KeyCode::Enter));

    assert_eq!(degrees_of(&screen, 0), "I-V-VIm-IV");
    assert_eq!(degrees_of(&screen, 1), "I-IV");
}

/// 打った文字列は 1 文字も変えずに入る（`-` で割って組み直したりしない）。
#[test]
fn the_typed_text_is_stored_verbatim() {
    for typed in ["C-7", "I-V | VIm-IV", "IIm7-V7-IMaj7"] {
        let mut screen = two_section_screen();
        screen.handle_key_event(key('i'));
        clear_input(&mut screen);
        type_text(&mut screen, typed);
        screen.handle_key_event(plain(KeyCode::Enter));

        assert!(!screen.line_input_open());
        assert_eq!(degrees_of(&screen, 0), typed);
    }
}
