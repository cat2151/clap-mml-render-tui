use ratatui::{backend::TestBackend, layout::Rect, Terminal};

use super::*;
use crate::CycleRandom;

const AREA: Rect = Rect {
    x: 0,
    y: 0,
    width: 90,
    height: 24,
};

fn render(screen: &GridSequencerScreen) -> String {
    let mut terminal = Terminal::new(TestBackend::new(AREA.width, AREA.height)).unwrap();
    let connection = screen.connection_status();
    terminal
        .draw(|f| crate::ui::draw(screen, &connection, f))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn screen() -> GridSequencerScreen {
    GridSequencerScreen::with_track_count(None, 4)
}

#[test]
fn the_overlay_lists_every_item_with_its_state() {
    let mut screen = screen();
    screen.toggle_cycle_random_overlay();
    screen.set_cycle_random(CycleRandomItem::Note, false);

    let rendered = render(&screen);

    for item in CycleRandomItem::ALL {
        assert!(rendered.contains(item.label()), "{rendered}");
    }
    assert!(rendered.contains("[x] 1 PATCH"), "{rendered}");
    assert!(rendered.contains("[ ] 2 NOTE"), "{rendered}");
    // 全角文字はセル2つぶんを占めるため、空白を無視して探す。
    assert!(
        rendered.replace(' ', "").contains("1周ごとのrandom"),
        "{rendered}"
    );
}

#[test]
fn the_overlay_is_hidden_until_it_is_opened() {
    let screen = screen();

    assert!(!render(&screen).contains("[x] 1 PATCH"));
}

/// 描画と当たり判定が同じ矩形を見ていないと、見えている行と切り替わる行がずれる。
#[test]
fn a_click_on_a_row_hits_the_item_drawn_there() {
    let overlay = overlay_rect(AREA);

    for (index, _) in CycleRandomItem::ALL.iter().enumerate() {
        let row = overlay.y + 1 + index as u16;
        assert_eq!(hit_test(AREA, overlay.x + 3, row), Some(Some(index)));
    }
    // 枠線と、項目より下の操作説明。
    assert_eq!(hit_test(AREA, overlay.x + 3, overlay.y), Some(None));
    assert_eq!(
        hit_test(
            AREA,
            overlay.x + 3,
            overlay.y + 1 + CycleRandomItem::ALL.len() as u16
        ),
        Some(None)
    );
    // 枠の外。
    assert_eq!(hit_test(AREA, 0, 0), None);
}

#[test]
fn clicking_a_row_toggles_that_item_and_moves_the_cursor() {
    let mut screen = screen();
    screen.toggle_cycle_random_overlay();
    let overlay = overlay_rect(AREA);

    screen.handle_cycle_random_click(overlay.x + 3, overlay.y + 1 + 2, AREA);

    assert!(!screen.cycle_random().drum, "3番目は DRUM");
    assert_eq!(screen.cycle_random_cursor(), Some(2));
    assert!(screen.cycle_random_open(), "click では閉じない");
}

#[test]
fn clicking_outside_the_frame_closes_the_overlay() {
    let mut screen = screen();
    screen.toggle_cycle_random_overlay();

    screen.handle_cycle_random_click(0, 0, AREA);

    assert!(!screen.cycle_random_open());
    assert_eq!(screen.cycle_random(), CycleRandom::ALL, "設定は動かさない");
}
