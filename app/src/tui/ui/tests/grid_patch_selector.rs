use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

/// 実 catalog cache（`cmrt build-patch-catalog-cache` の生成物）へのパス。無ければ下のテストは何もしない。
const REAL_CATALOG_ENV: &str = "CMRT_TEST_PATCH_CATALOG_JSON";

const WIDTH: u16 = 200;
const HEIGHT: u16 = 50;

fn symbol(buffer: &Buffer, x: u16, y: u16) -> String {
    buffer.cell((x, y)).unwrap().symbol().to_string()
}

fn line(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| symbol(buffer, x, y))
        .collect()
}

/// popup の外枠と 3 pane の縦枠が、pane の上辺から下辺まで全行そろっていること。
fn assert_frames_intact(buffer: &Buffer) {
    let pane_top = (0..HEIGHT)
        .find(|y| line(buffer, *y).contains(" Role "))
        .expect("Role pane の上辺がある");
    let corners = |y: u16| {
        (0..WIDTH)
            .filter(|x| matches!(symbol(buffer, *x, y).as_str(), "┌" | "┐"))
            .collect::<Vec<_>>()
    };
    let pane_columns = corners(pane_top);
    assert_eq!(pane_columns.len(), 6, "3 pane の角: {pane_columns:?}");
    let popup_columns = corners(pane_top - 1);
    assert_eq!(popup_columns.len(), 2, "外枠の角: {popup_columns:?}");
    let body_rows = (pane_top + 1..HEIGHT)
        .take_while(|y| symbol(buffer, pane_columns[0], *y) == "│")
        .inspect(|y| {
            for x in pane_columns.iter().chain(&popup_columns) {
                assert_eq!(symbol(buffer, *x, *y), "│", "x={x} y={y}");
            }
        })
        .count();
    assert!(body_rows >= 5, "{body_rows}");
    let pane_bottom = pane_top + 1 + u16::try_from(body_rows).unwrap();
    for x in &pane_columns {
        assert!(
            matches!(symbol(buffer, *x, pane_bottom).as_str(), "└" | "┘"),
            "x={x} y={pane_bottom}"
        );
    }
}

/// 実 catalog は preset label に `›` のような幅 1 の非 ASCII を含み、件数も合成データより
/// 3 桁多い。200 桁で popup の枠が崩れず、件数が共有の Role 索引と一致することを見る。
#[test]
fn grid_patch_selector_renders_the_real_catalog_cache_at_200_columns() {
    let Some(path) = std::env::var_os(REAL_CATALOG_ENV) else {
        return;
    };
    let (snapshot, _) = crate::patch_catalog_cache::load_from(std::path::Path::new(&path))
        .unwrap()
        .into_parts();
    let total = snapshot.pairs().len();
    let bass = snapshot
        .patch_roles()
        .candidates(cmrt_patches::PatchRole::Bass)
        .len();
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = std::sync::Arc::new(std::sync::Mutex::new(
        crate::tui::PatchLoadState::Ready(std::sync::Arc::new(snapshot)),
    ));
    app.active_screen = crate::screen_switch::PrimaryScreen::GridSequencer;

    let buffer = render_buffer(&mut app, WIDTH, HEIGHT);
    let (patch_x, header_y) = find_text(&buffer, "PATCH");
    app.handle_grid_sequencer_mouse_event(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: patch_x,
            row: header_y + 1,
            modifiers: KeyModifiers::NONE,
        },
        Rect::new(0, 0, WIDTH, HEIGHT),
    );

    let buffer = render_buffer(&mut app, WIDTH, HEIGHT);
    assert_frames_intact(&buffer);
    let screen = render_lines(&mut app, WIDTH, HEIGHT).join("\n");
    assert!(screen.contains("instance 1 patch select"), "{screen}");
    assert!(screen.contains(&format!("/{total}/{total})")), "{screen}");
    assert!(screen.contains("Bass › bass|bs"), "{screen}");
    assert!(screen.contains("Category"), "{screen}");

    for key in ['h', 'h', 'j'] {
        app.handle_grid_sequencer_key_event(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE));
    }
    let buffer = render_buffer(&mut app, WIDTH, HEIGHT);
    assert_frames_intact(&buffer);
    let screen = render_lines(&mut app, WIDTH, HEIGHT).join("\n");
    assert!(screen.contains("▶ Bass track"), "{screen}");
    assert!(screen.contains(&format!("/{bass}/{total})")), "{screen}");
}
