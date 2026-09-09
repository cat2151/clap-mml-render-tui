use super::*;

/// 端末が小さくても落ちない。列幅が負にならないことの実測。
#[test]
fn a_tiny_terminal_still_draws_without_panicking() {
    let screen = screen_with(song_of_eight_rows());

    for (width, height) in [(1, 1), (4, 3), (20, 6), (40, 10)] {
        let buffer = render_sized(&screen, width, height);
        assert_eq!(buffer.area.width, width);
        assert_eq!(buffer.area.height, height);
    }
}

/// ヘルプ overlay も同じく小さい端末で落ちない。
#[test]
fn a_tiny_terminal_still_draws_the_help_overlay() {
    let mut screen = screen_with(song_of_eight_rows());
    screen.help_open = true;

    for (width, height) in [(1, 1), (20, 6), (40, 10)] {
        render_sized(&screen, width, height);
    }
}

/// 2 つの pane は重ならず、横に並ぶ。
#[test]
fn the_two_panes_sit_side_by_side_without_overlapping() {
    let layout = layout_for(Rect::new(0, 0, 80, 24));

    assert_eq!(layout.sections.y, layout.arrangement.y);
    assert_eq!(layout.sections.height, layout.arrangement.height);
    assert_eq!(
        layout.sections.x + layout.sections.width,
        layout.arrangement.x
    );
    assert!(layout.header.y < layout.sections.y);
    assert!(layout.status.y > layout.sections.y + layout.sections.height - 1);
}
