//! `G` = chord wizard のテスト。
//!
//! 生成 MML の期待値は、`Cargo.lock` と同じ revision のローカル `chord2mml.exe` に
//! **実際に同じ入力を通して得た出力**をそのまま書いている（推測で書かない）。
//!
//! ```text
//! $ chord2mml.exe "close | I |"   → v11/*|*/'c1eg'/*|*/
//! $ chord2mml.exe "close | IV |"  → v11/*|*/'f1a<c'/*|*/
//! $ chord2mml.exe "close | V |"   → v11/*|*/'g1b<d'/*|*/
//! ```
//!
//! **1 セルに進行をまるごと書いた場合の出力も併記しておく。** これは wizard が
//! 書いてはいけない形（1 小節に押し込むと時間軸が 1/4 に圧縮される）。
//!
//! ```text
//! $ chord2mml.exe "close | I-IV-V-I |"
//! v11/*|*/'c4eg''f4a<c''g4b<d''c4eg'/*|*/
//! ```

use super::*;

use crate::{CHORD_TRACK, FIRST_PLAYABLE_TRACK};

/// wizard の対象にする演奏 track（グリッド行 index）。
pub(super) const TRACK: usize = FIRST_PLAYABLE_TRACK;
/// wizard が書き始める小節。init 列（0）ではない。row 全体の操作なので、
/// カーソルがどこにあっても常にここから配る。
pub(super) const MEASURE: usize = 1;

/// テスト用 grid の小節数。本番の `MEASURES` と同じ 8。
const WIDE_MEASURES: usize = 8;

/// wizard が init セルへ書く JSON。キーは serde_json の Map（BTreeMap）順、
/// つまり `"Surge XT patch"` が `"generate from chord track"` より先に出る。
const INIT_WITH_PATCH: &str =
    r#"{"Surge XT patch":"Pad 1.fxp","generate from chord track":"close"}"#;

/// crossterm は Shift 付きの大文字として届けるので、テストもその形で送る。
pub(super) fn press_g(app: &mut DawApp) {
    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
}

fn press_u(app: &mut DawApp) {
    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE));
}

pub(super) fn last_log(app: &DawApp) -> Option<String> {
    app.log_lines.lock().unwrap().back().cloned()
}

pub(super) fn set_catalog(app: &mut DawApp, progressions: &[&str]) {
    let progressions: Vec<String> = progressions.iter().map(|s| (*s).to_string()).collect();
    app.chord_progression_source = Some(Arc::new(move || progressions.clone()));
}

pub(super) fn cursor_at_wizard_target(app: &mut DawApp) {
    app.editor.cursor_track = TRACK;
    app.editor.cursor_measure = MEASURE;
}

/// `build_test_app` の grid は 2 小節しかないので、進行を配れるだけの幅を持つ
/// grid を作る。wizard は row 全体を書き換える操作なので、2 小節では
/// 「4 コードが 4 小節に散る」ことを確かめられない。
pub(super) fn build_wide_test_app() -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = build_test_app();
    let tracks = app.editor.data.len();
    app.editor = crate::editor::DawEditorState::new(
        vec![vec![String::new(); WIDE_MEASURES + 1]; tracks],
        FIRST_PLAYABLE_TRACK,
        MEASURE,
        tracks,
        WIDE_MEASURES,
    );
    // cache は editor とは別の grid を持つので、同じ幅へ揃えないと
    // `invalidate_cell` が範囲外を触る。
    *app.cache.lock().unwrap() = vec![vec![crate::CellCache::empty(); WIDE_MEASURES + 1]; tracks];
    (app, cache_rx)
}

/// chord 行の演奏小節（init 列を除く）を並べて返す。
pub(super) fn chord_row(app: &DawApp) -> Vec<String> {
    app.editor.data[CHORD_TRACK][1..].to_vec()
}

/// 対象 track の演奏小節を並べて返す。
fn track_row(app: &DawApp) -> Vec<String> {
    app.editor.data[TRACK][1..].to_vec()
}

#[test]
fn the_wizard_spreads_one_chord_per_measure_across_the_chord_row() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_at_wizard_target(&mut app);

    app.apply_chord_wizard_with("I-IV-V-I", Some("Pad 1.fxp".to_string()));

    // 進行まるごとを 1 セルへ書くと 1 小節に押し込まれて時間軸が 1/4 になる。
    assert_eq!(chord_row(&app), ["I", "IV", "V", "I", "", "", "", ""]);
    assert_eq!(app.editor.data[TRACK][0], INIT_WITH_PATCH);
    // 和音を配った小節の手書きだけ空にする。
    assert_eq!(track_row(&app), ["", "", "", "", "", "", "", ""]);
}

/// chord 行は row ごと差し替える。前の進行のほうが長かったときに尻尾が残ると、
/// 見えている進行と鳴るものが食い違う。
#[test]
fn a_shorter_progression_clears_the_tail_of_the_previous_one() {
    // `apply_chord_wizard_with` は既存セルを差し替えるので
    // `invalidate_dependent_cells`（`daw/src/cache.rs:203`）が
    // `track{行}_meas{小節}.wav` を消しに行く。guard 無しで走ると
    // 隣のテストの WAV を消す。
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("wizard_shorter_progression");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_at_wizard_target(&mut app);
    app.apply_chord_wizard_with("I-IV-V-I", None);

    app.apply_chord_wizard_with("I-V-vi", None);

    // `vi` が `VIm` になるのは chord2mml の正規化（鳴る音は同じ）。
    // [`crate::mml::chord_generation::split_progression_into_measures`] 参照。
    assert_eq!(chord_row(&app), ["I", "V", "VIm", "", "", "", "", ""]);
}

/// 和音を配っていない小節の手書きは消さない。置き換える和音が無いので、
/// 消せばただの破壊にしかならない。
#[test]
fn handwritten_cells_beyond_the_progression_are_left_alone() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_at_wizard_target(&mut app);
    app.editor.data[TRACK][1] = "cde".to_string();
    app.editor.data[TRACK][5] = "efg".to_string();

    app.apply_chord_wizard_with("I-IV-V", None);

    assert_eq!(track_row(&app), ["", "", "", "", "efg", "", "", ""]);
}

/// カーソルがどこにあっても meas.1 から配り、カーソル自身もそこへ移る
/// （書いた場所と画面が指す場所を合わせる）。
#[test]
fn the_wizard_always_starts_at_the_first_measure_and_moves_the_cursor_there() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    app.editor.cursor_track = TRACK;
    app.editor.cursor_measure = 6;

    app.apply_chord_wizard_with("I-IV", None);

    assert_eq!(chord_row(&app), ["I", "IV", "", "", "", "", "", ""]);
    assert_eq!(app.editor.cursor_measure, MEASURE);
}

/// 音色を既に選んである track の音色は変えない（wizard は和音を足しに来ただけ）。
#[test]
fn the_wizard_keeps_an_existing_patch_and_only_adds_the_generate_key() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    cursor_at_wizard_target(&mut app);
    app.editor.data[TRACK][0] =
        r#"{"Surge XT patch": "Chosen.fxp", "Surge XT patch filter": "pad"}"#.to_string();

    app.apply_chord_wizard_with("I-IV", None);

    assert_eq!(
        app.editor.data[TRACK][0],
        r#"{"Surge XT patch":"Chosen.fxp","Surge XT patch filter":"pad","generate from chord track":"close"}"#
    );
}

/// 手書きは chord 行より優先される（資料 4.5）。消さないと wizard を押しても
/// 音が変わらないので、patch history へ退避してから空にする。
#[test]
fn the_wizard_files_the_handwritten_cell_into_patch_history_before_clearing_it() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_test_app();
    cursor_at_wizard_target(&mut app);
    app.editor.data[TRACK][0] = r#"{"Surge XT patch": "Chosen.fxp"}"#.to_string();
    app.editor.data[TRACK][MEASURE] = "cde".to_string();

    app.apply_chord_wizard_with("I-IV", None);

    assert_eq!(app.editor.data[TRACK][MEASURE], "");
    assert_eq!(
        app.patch_phrase_store
            .patches
            .get("Chosen.fxp")
            .map(|state| state.history.clone()),
        Some(vec!["cde".to_string()])
    );
}

/// wizard が書いた結果が、実際に演奏経路の MML になるところまで見る。
/// 1 小節につき 1 和音（全音符）で、小節ごとに進行どおり変わる。
#[test]
fn the_track_the_wizard_marked_plays_one_chord_per_measure() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_at_wizard_target(&mut app);
    app.editor.data[0][0] = crate::DEFAULT_TRACK0_MML.to_string();

    app.apply_chord_wizard_with("I-IV-V-I", Some("Pad 1.fxp".to_string()));

    for (measure, expected) in [
        (1, r#"t120v11/*|*/'c1eg'/*|*/"#),
        (2, r#"t120v11/*|*/'f1a<c'/*|*/"#),
        (3, r#"t120v11/*|*/'g1b<d'/*|*/"#),
        (4, r#"t120v11/*|*/'c1eg'/*|*/"#),
    ] {
        let mml = crate::mml::build_cell_mml_from_data(
            &app.editor.data,
            app.editor.measures,
            TRACK,
            measure,
        );
        assert!(mml.ends_with(expected), "measure {measure} mml: {mml}");
    }
}

/// **時間軸の検証。** 1 小節の中で音を数えるだけでは、進行が 1 小節へ圧縮されても
/// 気づけない（それが最初の実装の見落とし）。小節をまたいで NoteOn の絶対 tick を
/// 並べ、和音が小節の頭に 1 つずつ立っていることを見る。
///
/// 圧縮されていると meas.1 が
/// `[(0,60),(0,64),(0,67),(480,65),…,(1440,67)]` の 12 個になり、meas.2 以降が
/// 空になるので、このテストが落ちる。
#[test]
fn each_chord_lands_at_the_head_of_its_own_measure() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_at_wizard_target(&mut app);
    app.editor.data[0][0] = crate::DEFAULT_TRACK0_MML.to_string();

    app.apply_chord_wizard_with("I-IV-V-I", Some("Pad 1.fxp".to_string()));

    let expected = [
        vec![(0, 60), (0, 64), (0, 67)],
        vec![(0, 65), (0, 69), (0, 72)],
        vec![(0, 67), (0, 71), (0, 74)],
        vec![(0, 60), (0, 64), (0, 67)],
    ];
    for (index, expected) in expected.iter().enumerate() {
        let measure = index + 1;
        let mml = crate::mml::build_cell_mml_from_data(
            &app.editor.data,
            app.editor.measures,
            TRACK,
            measure,
        );
        assert_eq!(
            &crate::mml::tests::chord_note_counts::note_ons(&mml),
            expected,
            "measure {measure}"
        );
    }
    // 進行より後ろの小節は無音。
    let mml = crate::mml::build_cell_mml_from_data(&app.editor.data, app.editor.measures, TRACK, 5);
    assert!(crate::mml::tests::chord_note_counts::note_ons(&mml).is_empty());
}

/// chord 行 init の `key:` も生成に効く（wizard は key を書かないので、
/// 曲全体の key は chord 行 init に書いたものがそのまま生きる）。
#[test]
fn the_key_on_the_chord_row_init_still_applies_to_what_the_wizard_wrote() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_at_wizard_target(&mut app);
    app.editor.data[0][0] = crate::DEFAULT_TRACK0_MML.to_string();
    app.editor.data[CHORD_TRACK][0] = "key:G".to_string();

    app.apply_chord_wizard_with("I-IV", Some("Pad 1.fxp".to_string()));

    for (measure, expected) in [
        (1, r#"t120v11/*|*/'g1b<d'/*|*/"#),
        (2, r#"t120v11/*|*/'<c1eg'/*|*/"#),
    ] {
        let mml = crate::mml::build_cell_mml_from_data(
            &app.editor.data,
            app.editor.measures,
            TRACK,
            measure,
        );
        assert!(mml.ends_with(expected), "measure {measure} mml: {mml}");
    }
}

#[test]
fn undo_restores_every_cell_the_wizard_wrote() {
    // `apply_chord_wizard_with` は既存セルを差し替えるので
    // `invalidate_dependent_cells`（`daw/src/cache.rs:203`）が
    // `track{行}_meas{小節}.wav` を消しに行く。guard 無しで走ると
    // 隣のテストの WAV を消す。
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("wizard_undo_restores");
    let (mut app, _cache_rx) = build_test_app();
    cursor_at_wizard_target(&mut app);
    app.editor.data[TRACK][0] = r#"{"Surge XT patch": "Chosen.fxp"}"#.to_string();
    app.editor.data[TRACK][MEASURE] = "cde".to_string();
    app.editor.data[CHORD_TRACK][MEASURE] = "V".to_string();

    app.apply_chord_wizard_with("I-IV", None);
    press_u(&mut app);

    assert_eq!(app.editor.data[CHORD_TRACK][MEASURE], "V");
    assert_eq!(
        app.editor.data[TRACK][0],
        r#"{"Surge XT patch": "Chosen.fxp"}"#
    );
    assert_eq!(app.editor.data[TRACK][MEASURE], "cde");
    assert!(app.editor.cell_undo.is_none());
}

/// 取り消しは、取り消しと無関係な編集を巻き込まない。
#[test]
fn undo_leaves_alone_a_cell_that_was_edited_after_the_wizard() {
    // `apply_chord_wizard_with` は既存セルを差し替えるので
    // `invalidate_dependent_cells`（`daw/src/cache.rs:203`）が
    // `track{行}_meas{小節}.wav` を消しに行く。guard 無しで走ると
    // 隣のテストの WAV を消す。
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("wizard_undo_unrelated_edit");
    let (mut app, _cache_rx) = build_test_app();
    cursor_at_wizard_target(&mut app);
    app.editor.data[CHORD_TRACK][MEASURE] = "V".to_string();

    app.apply_chord_wizard_with("I-IV", None);
    app.editor.data[CHORD_TRACK][MEASURE] = "vi-IV".to_string();
    press_u(&mut app);

    assert_eq!(app.editor.data[CHORD_TRACK][MEASURE], "vi-IV");
    // 触っていない init セルのほうはちゃんと戻る。
    assert_eq!(app.editor.data[TRACK][0], "");
}

/// 同じ結果を 2 回書いても、2 回目は何も変えない（`g` と同じ no-op 規則）。
#[test]
fn running_the_wizard_twice_with_the_same_pick_changes_nothing() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = build_wide_test_app();
    cursor_at_wizard_target(&mut app);

    app.apply_chord_wizard_with("I-IV", None);
    app.editor.cell_undo = None;
    app.apply_chord_wizard_with("I-IV", None);

    assert!(app.editor.cell_undo.is_none());
}
