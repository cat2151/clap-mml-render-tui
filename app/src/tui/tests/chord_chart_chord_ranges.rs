//! 「行内のどこからどこまでが 1 つの chord か」の写しが、glue から画面へ
//! **追随して**書き戻されるか（chord の個数もこの写しの長さから引く）。
//!
//! 切り方そのもの（`chord_source_ranges`）は `cmrt-chord` のテストが、写しの引き方は
//! `chord-chart` crate のテストが見る。ここで見るのは **キーを実際に打ったあと**の
//! 写しの中身と長さ。画面 crate は degrees を切れない（ADR 0020）ので、
//! ここが繋がっていないと chord カーソルは永久に 0 番から動かないし、反転も出ない。
//!
//! 保存が走るキー（`i` `dd`）を打つので、保存先は `set_local_dir_envs` で temp へ隔離する。

use super::chord_chart_preview::app_on_the_chord_chart;
use super::*;
use crate::screen_switch::PrimaryScreen;

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// 保存先を temp へ隔離した chord chart 画面（section 2 つ・arrangement 1 行）。
/// 返り値の guard を落とすと環境変数が戻るので、テストの最後まで持っておくこと。
fn app_with_isolated_save<'a>() -> (
    TuiApp<'a>,
    std::path::PathBuf,
    crate::test_utils::TestEnvGuard,
) {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_chord_counts_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let guard = crate::test_utils::set_local_dir_envs(&tmp);
    (app_on_the_chord_chart(), tmp, guard)
}

/// 曲の全 section ぶんの写し（名前 → chord 数）。
fn counts(app: &TuiApp<'_>) -> Vec<(String, Option<usize>)> {
    app.chord_chart
        .song
        .sections
        .iter()
        .map(|section| {
            (
                section.name.clone(),
                app.chord_chart.chord_count(section.id),
            )
        })
        .collect()
}

/// 画面へ入った時点で写しが入っていること（`switch_to_primary_screen` 経由）。
#[test]
fn entering_the_screen_writes_the_counts_back() {
    let app = app_on_the_chord_chart();

    assert_eq!(
        counts(&app),
        vec![("A".to_string(), Some(4)), ("B".to_string(), Some(4)),],
        "`I-V-VIm-IV` と `IIm-V-I-VIm` はどちらも 4 chord"
    );
    assert_eq!(app.chord_chart.cursor_chord_count(), 4);
}

/// **`i` で degrees を打ち替えると写しが追随する。** キーを実際に打って確かめる
/// （画面が数えるのではなく、glue が数え直して書き戻すのが正しい経路）。
#[test]
fn the_counts_follow_a_hand_typed_degrees() {
    let (mut app, tmp, _guard) = app_with_isolated_save();
    assert_eq!(app.chord_chart.cursor_chord_count(), 4);

    // `i` は今の degrees を入れて開く。消してから 2 chord ぶんを打ち直す。
    app.handle_chord_chart_key_event(plain(KeyCode::Char('i')));
    for _ in 0..20 {
        app.handle_mml_overlay_key_event(plain(KeyCode::Backspace));
    }
    for ch in "IIm-V".chars() {
        app.handle_mml_overlay_key_event(plain(KeyCode::Char(ch)));
    }
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));

    assert_eq!(app.chord_chart.song.sections[0].degrees, "IIm-V");
    assert_eq!(
        counts(&app),
        vec![("A".to_string(), Some(2)), ("B".to_string(), Some(4)),],
        "打ち替えた行だけが 2 になり、他の行はそのまま"
    );
    assert_eq!(app.chord_chart.cursor_chord_count(), 2);
    std::fs::remove_dir_all(&tmp).ok();
}

/// 読めない degrees を打つと 0 件になる。**画面は落ちず、chord カーソルは 1 個扱い**
/// （＝行全体を鳴らす側へ倒れる。`!` 印も赤字も出さない。ADR 0020）。
#[test]
fn an_unreadable_degrees_counts_zero_and_stays_a_single_chord() {
    let (mut app, tmp, _guard) = app_with_isolated_save();

    app.handle_chord_chart_key_event(plain(KeyCode::Char('i')));
    for _ in 0..20 {
        app.handle_mml_overlay_key_event(plain(KeyCode::Backspace));
    }
    for ch in "@@@".chars() {
        app.handle_mml_overlay_key_event(plain(KeyCode::Char(ch)));
    }
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));

    assert_eq!(app.chord_chart.song.sections[0].degrees, "@@@");
    assert_eq!(counts(&app)[0].1, Some(0), "数えた結果の 0 は 0 のまま");
    assert_eq!(app.chord_chart.cursor_chord_count(), 1);
    // `i` の確定では preview 要求が立たない（要求はカーソルが動いたときだけ）ので、
    // 読めない degrees を打っても下段に理由は出ない。**画面は落ちない**ことが要点。
    assert_eq!(app.chord_chart.error, None);
    std::fs::remove_dir_all(&tmp).ok();
}

/// `dd` で section を消したら、**写しの長さも曲に合う**（消した行の答えが残らない）。
#[test]
fn deleting_a_section_shrinks_the_copy_to_match_the_song() {
    let (mut app, tmp, _guard) = app_with_isolated_save();
    assert_eq!(app.chord_chart.chord_ranges_len(), 2);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('d')));
    app.handle_chord_chart_key_event(plain(KeyCode::Char('d')));

    assert_eq!(app.chord_chart.song.sections.len(), 1);
    assert_eq!(
        app.chord_chart.chord_ranges_len(),
        1,
        "消した section の答えが写しに残らないこと"
    );
    assert_eq!(counts(&app), vec![("B".to_string(), Some(4))]);
    std::fs::remove_dir_all(&tmp).ok();
}

/// `g` で足した section も、押したその場で写しへ入る（数え直しは曲が変わるたび）。
#[test]
fn a_section_added_from_the_catalog_is_counted_right_away() {
    let (mut app, tmp, _guard) = app_with_isolated_save();
    app.chord_chart
        .set_chord_progression_source(std::sync::Arc::new(|| vec!["I-IV".to_string()]));

    app.handle_chord_chart_key_event(plain(KeyCode::Char('g')));

    assert_eq!(app.chord_chart.song.sections.len(), 3);
    assert_eq!(counts(&app)[2], ("C".to_string(), Some(2)));
    // カーソルは足した行にいる（`add_section_from_catalog`）。
    assert_eq!(app.chord_chart.cursor_chord_count(), 2);
    std::fs::remove_dir_all(&tmp).ok();
}

/// 書き戻される**前**でも画面は落ちず、chord カーソルは 0 番のまま
/// （＝ `cursor_chord_count` が 1）。写しを空にした状態で描いても落ちないこと。
#[test]
fn a_screen_without_the_copy_still_reports_a_single_chord() {
    let (mut app, tmp, _guard) = app_with_isolated_save();

    // 誰も書き戻していない状態を作る（新品の画面と同じ）。
    app.chord_chart.set_chord_ranges(std::iter::empty());

    assert_eq!(app.chord_chart.chord_ranges_len(), 0);
    assert_eq!(
        app.chord_chart.cursor_chord_count(),
        1,
        "写しが無い行は chord 1 個扱い＝ chord カーソルは 0 番から動けない"
    );
    // 描画も落ちない（写しがあることを描画側が前提にしていない）。
    let lines = crate::tui::ui::tests::render_lines(&mut app, 80, 24);
    assert!(lines.join("\n").contains("I-V-VIm-IV"));

    // 画面へ入り直せば埋まる。
    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    assert_eq!(app.chord_chart.cursor_chord_count(), 4);
    std::fs::remove_dir_all(&tmp).ok();
}

/// **写しと chord カーソルが繋がっていること**を、キーだけで確かめる。
///
/// 画面 crate は degrees を数えられないので、glue の書き戻しが届いていなければ
/// カーソル行の chord 数は 1 になり、`l` の 1 回目でいきなり次の行へ繰り上がる。
/// 4 回目でようやく繰り上がることが、写しが本物の 4 件だという証拠になる。
#[test]
fn the_chord_cursor_walks_the_counted_chords_before_carrying_over() {
    let mut app = app_on_the_chord_chart();
    assert_eq!(app.chord_chart.cursor_chord_count(), 4);

    for expected in 1..=3 {
        app.handle_chord_chart_key_event(plain(KeyCode::Char('l')));
        assert_eq!(app.chord_chart.chord_cursor(), expected);
        assert_eq!(
            app.chord_chart.clamped_section_cursor(),
            0,
            "4 chord ぶんは行内で動くこと"
        );
    }

    app.handle_chord_chart_key_event(plain(KeyCode::Char('l')));

    assert_eq!(
        app.chord_chart.clamped_section_cursor(),
        1,
        "次の行へ繰り上がる"
    );
    assert_eq!(app.chord_chart.chord_cursor(), 0);
    assert_eq!(app.chord_chart.error, None);
}

/// `i` で chord を減らして確定しても、chord カーソルが行の外に残らないこと
/// （確定では preview 要求が立たないので、丸めるのは読むときだけ）。
#[test]
fn shrinking_the_degrees_with_the_mml_overlay_pulls_the_chord_cursor_back() {
    let (mut app, tmp, _guard) = app_with_isolated_save();
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    for _ in 0..3 {
        app.handle_chord_chart_key_event(plain(KeyCode::Char('l')));
    }
    assert_eq!(app.chord_chart.chord_cursor(), 3);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('i')));
    for _ in 0..64 {
        app.handle_mml_overlay_key_event(plain(KeyCode::Backspace));
    }
    for code in ['I', '-', 'V'] {
        app.handle_mml_overlay_key_event(plain(KeyCode::Char(code)));
    }
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));

    assert_eq!(app.chord_chart.song.sections[0].degrees, "I-V");
    assert_eq!(
        app.chord_chart.cursor_chord_count(),
        2,
        "写しも 2 件へ追随する"
    );
    assert_eq!(app.chord_chart.chord_cursor(), 1, "末尾へ丸める");
    std::fs::remove_dir_all(&tmp).ok();
}

/// 反転している桁を全部拾って 1 つの文字列にする（chord chart は 1 行しか反転しない）。
fn reversed_text(app: &mut TuiApp<'static>) -> String {
    let buffer = crate::tui::ui::tests::render_buffer(app, 100, 24);
    (0..buffer.area.height)
        .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
        .filter(|position| {
            buffer
                .cell(*position)
                .unwrap()
                .style()
                .add_modifier
                .contains(ratatui::style::Modifier::REVERSED)
        })
        .map(|position| buffer.cell(position).unwrap().symbol().to_string())
        .collect()
}

/// **端から端まで**: 本物の `chord_source_ranges` が切った範囲で、本物の degrees が
/// 反転すること。crate 側のテストは写しをテスト用に組み立てているので、
/// 「本物の切り方だと桁がずれる」ことがあってもそちらでは出ない
/// （区切りのハイフンを範囲へ含めていないか、など）。
#[test]
fn the_real_ranges_reverse_exactly_the_chord_on_the_screen() {
    let (mut app, tmp, _guard) = app_with_isolated_save();
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    assert_eq!(app.chord_chart.song.sections[0].degrees, "I-V-VIm-IV");

    assert_eq!(reversed_text(&mut app), "I", "入った直後は先頭 chord");

    app.handle_chord_chart_key_event(plain(KeyCode::Char('l')));
    assert_eq!(reversed_text(&mut app), "V", "ハイフンは反転に入らない");

    app.handle_chord_chart_key_event(plain(KeyCode::Char('l')));
    assert_eq!(reversed_text(&mut app), "VIm");

    app.handle_chord_chart_key_event(plain(KeyCode::Char('l')));
    assert_eq!(reversed_text(&mut app), "IV", "末尾の chord");
    std::fs::remove_dir_all(&tmp).ok();
}
