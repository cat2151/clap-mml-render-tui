use super::*;
use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadState};

const BASS: &str = "patches_factory/Basses/Wobble Bass.fxp";
const LEAD: &str = "patches_factory/Leads/Screaming Lead.fxp";

fn patch_cell(display: &str) -> String {
    format!(r#"{{"Surge XT patch": "{display}"}}"#)
}

/// 音色の用途まで引ける catalog を app へ載せる。
fn load_catalog(app: &DawApp, displays: &[&str]) {
    let snapshot = PatchCatalogSnapshot::from_pairs(
        displays
            .iter()
            .map(|display| {
                (
                    (*display).to_string(),
                    cmrt_patches::normalize_patch_lookup_key(display),
                )
            })
            .collect(),
    );
    *app.patch_load.lock().unwrap() = PatchLoadState::Ready(std::sync::Arc::new(snapshot));
}

/// mixer overlay の枠の左上 (x, y)。
///
/// overlay は画面中央に開くので、その左右には下地の grid が残る。**行全体を検索すると
/// grid 側の `T1` を拾ってしまう**ので、overlay の内側だけを切り出して読むこと。
fn mixer_overlay_origin(buffer: &Buffer) -> (u16, u16) {
    // 画面外周の枠（0, 0）を飛ばして最初に見つかる角が mixer overlay。
    for y in 1..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer.cell((x, y)).unwrap().symbol() == "┌" {
                return (x, y);
            }
        }
    }
    panic!("mixer overlay not found");
}

/// overlay の内側の `row` 行目（0 = ヘッダ 1 行目）の文字列。
fn overlay_line(buffer: &Buffer, row: u16) -> String {
    let (left, top) = mixer_overlay_origin(buffer);
    let right = (left + 1..buffer.area.width)
        .find(|x| buffer.cell((*x, top)).unwrap().symbol() == "┐")
        .expect("mixer overlay has no right border");
    (left + 1..right)
        .map(|x| {
            buffer
                .cell((x, top + 1 + row))
                .unwrap()
                .symbol()
                .to_string()
        })
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// overlay 内で `text` が始まる (x, y)。
fn find_in_overlay(buffer: &Buffer, text: &str) -> (u16, u16) {
    let (left, top) = mixer_overlay_origin(buffer);
    for row in 0..buffer.area.height.saturating_sub(top + 1) {
        let line = overlay_line(buffer, row);
        if let Some(byte_index) = line.find(text) {
            let column = line[..byte_index].chars().count() as u16;
            return (left + 1 + column, top + 1 + row);
        }
    }
    panic!("text not found in mixer overlay: {text}");
}

fn mixer_app() -> DawApp {
    let mut app = build_test_app();
    app.mode = DawMode::Mixer;
    app.overlays.mixer.cursor_track = crate::FIRST_PLAYABLE_TRACK;
    app
}

#[test]
fn draw_shows_mixer_overlay_with_track_labels_and_db_values() {
    let mut app = mixer_app();
    app.track_volumes_db[2] = -3;
    app.track_volumes_db[3] = 6;

    let normalized_lines: Vec<String> = render_lines(&app, 100, 30)
        .into_iter()
        .map(|line| line.to_lowercase())
        .collect();

    assert!(
        normalized_lines.iter().any(|line| line.contains("mixer")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("-3db") && line.contains("+6db")),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn the_track_label_matches_the_grid_row_label() {
    // grid の行頭が `T1` なのに mixer が `track1` だと、同じ track を指していると読めない。
    let app = mixer_app();
    let buffer = render_buffer(&app, 100, 30);

    let header = overlay_line(&buffer, 0);
    assert!(
        header.contains("T1") && header.contains("T2"),
        "header: {header:?}"
    );
}

#[test]
fn the_header_shows_the_patch_role_and_the_patch_name_of_each_track() {
    let mut app = mixer_app();
    load_catalog(&app, &[BASS, LEAD]);
    app.editor.data[2][0] = patch_cell(BASS);
    app.editor.data[3][0] = patch_cell(LEAD);

    let buffer = render_buffer(&app, 100, 30);

    let roles = overlay_line(&buffer, 1);
    assert!(
        roles.contains("bass") && roles.contains("lead"),
        "roles: {roles:?}"
    );
    let patches = overlay_line(&buffer, 2);
    // 列幅に入らない音色名は末尾が切れる（`Screaming Lead` → `Screaming Lea`）。
    // 区切りの 1 桁は必ず残るので、隣の列とはくっつかない。
    assert!(
        patches.contains("Wobble Bass") && patches.contains("Screaming Lea"),
        "patches: {patches:?}"
    );
}

#[test]
fn a_track_without_a_patch_shows_the_missing_mark_instead_of_a_blank_column() {
    // 空欄だと「読み込み中」なのか「列がずれている」のか見分けが付かない。
    let app = mixer_app();
    let buffer = render_buffer(&app, 100, 30);

    assert!(overlay_line(&buffer, 1).contains("---"));
    assert!(overlay_line(&buffer, 2).contains("---"));
}

#[test]
fn a_generated_track_is_marked_in_the_role_row() {
    let mut app = mixer_app();
    load_catalog(&app, &[BASS]);
    app.editor.data[2][0] =
        format!(r#"{{"Surge XT patch": "{BASS}", "generate from chord track": "close"}}"#);

    let buffer = render_buffer(&app, 100, 30);

    assert!(overlay_line(&buffer, 1).contains("*bass"));
}

#[test]
fn draw_highlights_selected_mixer_track_with_contrast_background_without_blink() {
    let app = mixer_app();

    let buffer = render_buffer(&app, 100, 30);
    let highlighted_positions: Vec<(u16, u16)> = (0..100)
        .flat_map(|x| (0..30).map(move |y| (x, y)))
        .filter(|(x, y)| {
            let cell = buffer.cell((*x, *y)).unwrap();
            cell.bg == cursor_highlight_bg(cell.fg)
                && !cell
                    .modifier
                    .contains(ratatui::style::Modifier::RAPID_BLINK)
        })
        .collect();

    assert!(
        !highlighted_positions.is_empty(),
        "selected mixer track should use a contrast background"
    );

    let (x, y) = find_in_overlay(&buffer, "T1");
    let cell = buffer.cell((x, y)).unwrap();
    assert_eq!(cell.fg, MONOKAI_FG);
    assert_eq!(cell.bg, cursor_highlight_bg(MONOKAI_FG));
}
