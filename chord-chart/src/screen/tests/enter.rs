//! 画面へ入るとき（保存の復元と初回抽選）と、行が減ったあとのカーソル。

use super::*;

#[test]
fn the_screen_starts_on_the_sections_pane() {
    let screen = ChordChartScreen::new(one_section_song());

    assert_eq!(screen.focus, Pane::Sections);
    assert_eq!(screen.song.sections.len(), 1);
    assert_eq!(
        screen.selected_section().map(|s| s.name.as_str()),
        Some("A")
    );
}

/// 保存ファイルが読めた（`Some`）なら、画面へ入っても**何も足さない**。
#[test]
fn a_restored_song_is_left_exactly_as_it_was_loaded() {
    let saved = one_section_song();
    let mut screen = with_catalog(
        ChordChartScreen::restored(Some(saved.clone())),
        &["I-IV-V-I"],
    );

    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert_eq!(screen.song, saved);
}

/// section を全部消して保存した曲（`Some(空)`）も、そのまま空で開く。
/// ここで抽選すると、消したはずの section が再起動のたびに復活する。
#[test]
fn a_restored_song_with_no_sections_stays_empty() {
    let mut screen = with_catalog(
        ChordChartScreen::restored(Some(Song::empty())),
        &["I-IV-V-I"],
    );

    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert!(screen.song.sections.is_empty());
    assert!(screen.song.arrangement.is_empty());
}

/// 保存ファイルが読めなかった（`None`）ときだけ、`g` と同じ抽選が 1 回走る。
/// **進行はカタログから来る**（この crate はコード進行を 1 つも持たない）。
#[test]
fn a_song_that_could_not_be_loaded_generates_one_section_from_the_catalog() {
    let mut screen = with_catalog(ChordChartScreen::restored(None), &["I-IV-V-I"]);

    assert_eq!(screen.enter(), ChordChartAction::SongChanged);

    assert_eq!(screen.song.sections.len(), 1);
    assert_eq!(screen.song.sections[0].name, "A");
    assert_eq!(screen.song.sections[0].degrees, "I-IV-V-I");
    // 右 pane も 1 行から始まる（section だけだと並びが空のままになる）。
    assert_eq!(screen.song.arrangement, vec![screen.song.sections[0].id]);
    assert_eq!(screen.error, None);
}

/// 自動抽選は**入るたびには走らない**。画面を出入りするたびに section が増えると、
/// 曲が勝手に伸びていく。
#[test]
fn the_automatic_pick_happens_only_on_the_first_enter() {
    let mut screen = with_catalog(ChordChartScreen::restored(None), &["I-IV-V-I"]);

    assert_eq!(screen.enter(), ChordChartAction::SongChanged);
    assert_eq!(screen.enter(), ChordChartAction::Continue);
    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert_eq!(screen.song.sections.len(), 1);
}

/// カタログが無ければ空のまま。**進行をハードコードして埋め合わせない**。
/// 理由は下段に出す（無反応で終わらせない）。
#[test]
fn a_failed_automatic_pick_leaves_the_song_empty_with_a_reason() {
    let mut screen = ChordChartScreen::restored(None);

    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert!(screen.song.sections.is_empty());
    assert!(screen.song.arrangement.is_empty());
    assert_eq!(
        screen.error.as_deref(),
        Some(crate::catalog::NO_CATALOG_MESSAGE)
    );
    // 失敗しても「1 回だけ」は使い切る（開き直すたびに取得を待たされない）。
    assert_eq!(screen.enter(), ChordChartAction::Continue);
}

/// 空カタログ（取得はできたが 0 件）も同じ扱い。
#[test]
fn an_empty_catalog_is_treated_like_a_missing_one() {
    let mut screen = with_catalog(ChordChartScreen::restored(None), &[]);

    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert!(screen.song.sections.is_empty());
    assert_eq!(
        screen.error.as_deref(),
        Some(crate::catalog::NO_CATALOG_MESSAGE)
    );
}

/// 行が減ったあとに保持した index が溢れても、描画側が範囲外を引かない。
#[test]
fn a_cursor_past_the_end_is_clamped_to_the_last_row() {
    let screen = ChordChartScreen {
        section_cursor: 99,
        arrangement_cursor: 99,
        ..ChordChartScreen::new(one_section_song())
    };

    assert_eq!(screen.clamped_section_cursor(), 0);
    assert_eq!(screen.clamped_arrangement_cursor(), 0);
    assert!(screen.selected_section().is_some());
}

#[test]
fn an_empty_song_has_no_selected_section_and_a_zero_cursor() {
    let mut screen = ChordChartScreen::new(Song::empty());
    screen.section_cursor = 3;

    assert_eq!(screen.clamped_section_cursor(), 0);
    assert_eq!(screen.clamped_arrangement_cursor(), 0);
    assert!(screen.selected_section().is_none());
}
