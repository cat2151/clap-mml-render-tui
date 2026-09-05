use super::*;

#[test]
fn draw_shows_mml_and_uncached_dot_before_cache_is_ready() {
    let mut app = build_test_app();
    app.editor.data[2][1] = "cdef".to_string();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[2][1].state = CacheState::Pending;
    }

    let lines = render_lines(&app, 40, 19);

    assert!(
        lines.iter().any(|line| line.contains("cdef")),
        "lines: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|line| line.contains('.')),
        "lines: {:?}",
        lines
    );
}

#[test]
fn draw_renders_pending_indicator_in_visible_color() {
    let mut app = build_test_app();
    app.editor.data[2][1] = "cdef".to_string();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[2][1].state = CacheState::Pending;
    }

    let buffer = render_buffer(&app, 40, 19);
    let pending_indicator = (0..buffer.area.height)
        .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
        .find(|&(x, y)| {
            let cell = buffer.cell((x, y)).unwrap();
            cell.symbol() == "." && cell.fg == MONOKAI_FG
        });

    assert!(
        pending_indicator.is_some(),
        "buffer should contain a visible pending indicator"
    );
}

#[test]
fn draw_uses_contrast_background_for_selected_grid_cell_without_blink() {
    let mut app = build_test_app();
    app.editor.data[0][0] = "t120".to_string();

    let buffer = render_buffer(&app, 40, 18);
    let (x, y) = find_text_ignoring_spaces(&buffer, "t120");
    let cell = buffer.cell((x, y)).unwrap();

    assert_eq!(cell.fg, MONOKAI_GRAY);
    assert_eq!(cell.bg, cursor_highlight_bg(MONOKAI_GRAY));
    assert!(!cell
        .modifier
        .contains(ratatui::style::Modifier::RAPID_BLINK));
}

/// 指定 y 行の文字を 1 セル 1 要素で取り出す（多バイト記号も 1 要素）。
pub(super) fn row_symbols(buffer: &Buffer, y: u16) -> Vec<String> {
    (0..buffer.area.width)
        .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect()
}

/// 指定 y 行で `text` が始まる x 座標。見つからなければ panic。
pub(super) fn x_of_in_row(buffer: &Buffer, y: u16, text: &str) -> u16 {
    let symbols = row_symbols(buffer, y);
    let needle: Vec<String> = text.chars().map(|c| c.to_string()).collect();
    (0..symbols.len())
        .find(|&start| symbols[start..].starts_with(&needle[..]))
        .map(|x| x as u16)
        .unwrap_or_else(|| {
            panic!(
                "{text:?} not found in row {y}: {:?}",
                symbols.concat().trim_end()
            )
        })
}

/// `Init` を含むヘッダ行の y 座標。
pub(super) fn header_row(buffer: &Buffer) -> u16 {
    (0..buffer.area.height)
        .find(|&y| row_symbols(buffer, y).concat().contains("Init"))
        .expect("header row with Init")
}

#[test]
fn grid_header_cells_and_indicators_share_the_same_column_x() {
    let mut app = build_test_app();
    app.editor.data[2][1] = "cdef".to_string();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[2][1].state = CacheState::Pending;
    }

    let buffer = render_buffer(&app, 60, 19);
    let header_y = header_row(&buffer);
    // track 1 の本体行はヘッダの 1 行下から 2 行ずつ。
    let track1_y = header_y + 1 + 2 * crate::FIRST_PLAYABLE_TRACK as u16;

    let header_m1_x = x_of_in_row(&buffer, header_y, "M1");
    let cell_x = x_of_in_row(&buffer, track1_y, "cdef");
    let indicator_x = x_of_in_row(&buffer, track1_y + 1, ".");

    assert_eq!(header_m1_x, cell_x, "header and cell must line up");
    assert_eq!(
        header_m1_x, indicator_x,
        "header and indicator must line up"
    );
}

#[test]
fn init_column_occupies_fourteen_columns_and_measure_width_uses_available_space() {
    let buffer = render_buffer(&build_test_app(), 60, 15);
    let header_y = header_row(&buffer);

    let init_x = x_of_in_row(&buffer, header_y, "Init");
    let m1_x = x_of_in_row(&buffer, header_y, "M1");
    let m2_x = x_of_in_row(&buffer, header_y, "M2");

    assert_eq!(m1_x - init_x, 14, "init column must be 14 columns wide");
    assert_eq!(m2_x - m1_x, 9, "two measures can use the maximum width");
}

#[test]
fn ab_repeat_markers_do_not_shift_the_measure_columns() {
    let app = build_test_app();
    {
        let mut ab_repeat = app.playback.ab_repeat.lock().unwrap();
        *ab_repeat = AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        };
    }

    let buffer = render_buffer(&app, 60, 19);
    let header_y = header_row(&buffer);

    let init_x = x_of_in_row(&buffer, header_y, "Init");
    let a1_x = x_of_in_row(&buffer, header_y, "A1");
    let b2_x = x_of_in_row(&buffer, header_y, "B2");

    assert_eq!(a1_x - init_x, 14);
    assert_eq!(b2_x - a1_x, 9);
}

#[test]
fn init_cell_shows_more_than_four_characters() {
    let mut app = build_test_app();
    app.editor.data[2][0] = "0123456789abcdef".to_string();

    let buffer = render_buffer(&app, 60, 19);
    let header_y = header_row(&buffer);
    let track1_y = header_y + 1 + 2 * crate::FIRST_PLAYABLE_TRACK as u16;

    // 12 桁 + 省略記号まで出て、続きが隠れていることが分かる。
    x_of_in_row(&buffer, track1_y, "0123456789ab…");
    let row = row_symbols(&buffer, track1_y).concat();
    assert!(!row.contains("0123456789abc"), "row: {row:?}");
}

#[test]
fn every_measure_column_still_fits_in_an_80_column_terminal() {
    let mut app = build_test_app();
    let tracks = app.editor.tracks;
    app.editor = crate::editor::DawEditorState::new(
        vec![vec![String::new(); MEASURES + 1]; tracks],
        0,
        0,
        tracks,
        MEASURES,
    );
    app.cache = Arc::new(Mutex::new(vec![
        vec![CellCache::empty(); MEASURES + 1];
        tracks
    ]));
    app.editor.data[crate::CHORD_TRACK][1] = "IIm7(b5)".to_string();

    let buffer = render_buffer(&app, 80, 20);
    let header_y = header_row(&buffer);

    let init_x = x_of_in_row(&buffer, header_y, "Init");
    for m in 1..=MEASURES {
        let x = x_of_in_row(&buffer, header_y, &format!("M{m}"));
        assert_eq!(
            x as usize,
            init_x as usize + 14 + (m - 1) * 7,
            "M{m} must sit at its own column"
        );
    }
    let chord_y = header_y + 1 + 2 * crate::CHORD_TRACK as u16;
    assert_eq!(x_of_in_row(&buffer, chord_y, "IIm7(…"), init_x + 14);

    let wide_buffer = render_buffer(&app, 93, 20);
    let wide_header_y = header_row(&wide_buffer);
    let wide_chord_y = wide_header_y + 1 + 2 * crate::CHORD_TRACK as u16;
    let wide_m1_x = x_of_in_row(&wide_buffer, wide_header_y, "M1");
    assert_eq!(
        x_of_in_row(&wide_buffer, wide_chord_y, "IIm7(b5)"),
        wide_m1_x
    );
}

// ─── init 列（meas 0）の role:音色名 表示 ───────────────────

/// role が引ける音色を持つ catalog snapshot を app へ注入する。
pub(super) fn inject_catalog(app: &mut DawApp, displays: &[&str]) {
    let pairs = displays
        .iter()
        .map(|display| {
            (
                (*display).to_string(),
                cmrt_patches::normalize_patch_lookup_key(display),
            )
        })
        .collect();
    *app.patch_load.lock().unwrap() = cmrt_tui_core::patch_load::PatchLoadState::ready(pairs);
}

pub(super) fn patch_init_cell(display: &str) -> String {
    format!(r#"{{"Surge XT patch": "{display}"}}"#)
}

pub(super) const TEST_BASS_PATCH: &str = "patches_factory/Basses/Wobble Bass.fxp";
const TEST_LEAD_PATCH: &str = "patches_factory/Leads/Screaming Lead.fxp";

#[test]
fn init_column_shows_the_role_and_the_patch_name_per_track() {
    let mut app = build_test_app();
    app.editor.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
    app.editor.data[2][0] = patch_init_cell(TEST_BASS_PATCH);
    app.editor.data[3][0] = patch_init_cell(TEST_LEAD_PATCH);
    inject_catalog(&mut app, &[TEST_BASS_PATCH, TEST_LEAD_PATCH]);

    // track 2 の行まで描くには grid 領域の高さが要る。
    let lines = render_lines(&app, 60, 24);

    assert!(
        lines.iter().any(|line| line.contains("bass:Wobble…")),
        "lines: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("lead:Screami…")),
        "lines: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("4/4 t120")),
        "lines: {lines:?}"
    );
    // 訴えの元になっていた JSON の頭出しが消えていること。
    assert!(
        !lines.iter().any(|line| line.contains("{\"Su")),
        "lines: {lines:?}"
    );
}

#[test]
fn init_column_shows_the_patch_name_alone_while_the_catalog_is_loading() {
    let mut app = build_test_app();
    app.editor.data[2][0] = patch_init_cell(TEST_BASS_PATCH);

    let lines = render_lines(&app, 60, 19);

    assert!(
        lines.iter().any(|line| line.contains("Wobble Bass")),
        "lines: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("bass:")),
        "lines: {lines:?}"
    );
}

#[test]
fn init_column_keeps_showing_a_plain_mml_cell_as_is() {
    let mut app = build_test_app();
    app.editor.data[2][0] = "o4cdefgab".to_string();
    inject_catalog(&mut app, &[TEST_BASS_PATCH]);

    let lines = render_lines(&app, 60, 19);

    assert!(
        lines.iter().any(|line| line.contains("o4cdefgab")),
        "lines: {lines:?}"
    );
}

#[test]
fn init_column_truncates_a_long_role_and_patch_name_to_the_column_width() {
    let mut app = build_test_app();
    app.editor.data[2][0] = patch_init_cell(TEST_LEAD_PATCH);
    inject_catalog(&mut app, &[TEST_LEAD_PATCH]);

    let buffer = render_buffer(&app, 60, 19);
    let header_y = header_row(&buffer);
    let track1_y = header_y + 1 + 2 * crate::FIRST_PLAYABLE_TRACK as u16;

    // `lead:Screaming Lead` は省略記号込みの 13 桁で切られ、M1 列を侵食しない。
    let init_x = x_of_in_row(&buffer, header_y, "Init");
    let m1_x = x_of_in_row(&buffer, header_y, "M1");
    let row = row_symbols(&buffer, track1_y);
    let init_cell: String = (init_x..m1_x).map(|x| row[x as usize].clone()).collect();

    assert_eq!(init_cell, "lead:Screami… ");
}

/// 実機の config.toml を渡したときだけ走る、実カタログでの init 列表示。
///
/// 開発機のインストール状況に依存するので、環境変数が無ければ skip する
/// （個人のパスをコードへ書かないため、パスは環境変数で渡す）。
///
/// ```text
/// CMRT_TEST_DAW_REAL_CONFIG=%LOCALAPPDATA%\clap-mml-render-tui\config.toml ///   cargo test -p cmrt-daw real_catalog -- --nocapture
/// ```
#[test]
fn real_catalog_init_column_shows_role_prefixes() {
    let Some(config_path) = std::env::var_os("CMRT_TEST_DAW_REAL_CONFIG") else {
        eprintln!("skip: CMRT_TEST_DAW_REAL_CONFIG が未設定");
        return;
    };
    let cfg = Config::load_from_path(std::path::Path::new(&config_path))
        .expect("CMRT_TEST_DAW_REAL_CONFIG の config.toml を読めること");
    let pairs = cmrt_tui_core::patches::collect_patch_pairs(&cfg).expect("実カタログの走査");
    assert!(!pairs.is_empty(), "実機カタログが 0 件では検証にならない");

    let snapshot = cmrt_tui_core::patch_load::PatchCatalogSnapshot::from_pairs(pairs);
    // 実カタログから bass / lead に分類された音色を 1 件ずつ借りる。
    let bass = snapshot
        .patch_roles()
        .candidates(cmrt_patches::PatchRole::Bass)
        .first()
        .expect("実カタログに bass 音色があること")
        .clone();
    let lead = snapshot
        .patch_roles()
        .candidates(cmrt_patches::PatchRole::Lead)
        .first()
        .expect("実カタログに lead 音色があること")
        .clone();

    let mut app = build_test_app();
    app.cfg = Arc::new(cfg);
    app.editor.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
    app.editor.data[2][0] = patch_init_cell(&bass);
    app.editor.data[3][0] = patch_init_cell(&lead);
    *app.patch_load.lock().unwrap() =
        cmrt_tui_core::patch_load::PatchLoadState::Ready(Arc::new(snapshot));

    let lines = render_lines(&app, 60, 24);
    eprintln!("real catalog init column:");
    for line in lines.iter().take(7) {
        eprintln!("  |{line}|");
    }

    assert!(
        lines.iter().any(|line| line.contains("bass:")),
        "実カタログの bass 音色 {bass:?} が bass: と出るはず: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("lead:")),
        "実カタログの lead 音色 {lead:?} が lead: と出るはず: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("4/4 t120")),
        "lines: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("{\"Su")),
        "JSON の頭出しが残っている: {lines:?}"
    );
}

/// 行の並びは Tempo → Chord → T1 → T2。
///
/// chord 行が割り込んでも演奏 track のラベル番号はずれない
/// （保存ファイルの `"track": 1` と画面の `T1` が同じものを指し続ける）。
#[test]
fn the_chord_row_sits_between_the_tempo_row_and_the_first_playable_track() {
    let app = build_test_app();

    let lines = render_lines(&app, 60, 24);
    let labels: Vec<&str> = lines
        .iter()
        .filter_map(|line| {
            ["Tempo", "Chord", "T1", "T2"]
                .into_iter()
                .find(|label| line.trim_start_matches('│').starts_with(label))
        })
        .collect();

    assert_eq!(
        labels,
        vec!["Tempo", "Chord", "T1", "T2"],
        "lines: {lines:?}"
    );
}
