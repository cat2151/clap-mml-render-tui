use super::*;

#[test]
fn repeated_chord_voice_wheel_down_accumulates_and_survives_chord_changes() {
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    let before = screen.state.instances()[2]
        .lanes
        .iter()
        .map(|lane| lane.base_note)
        .collect::<Vec<_>>();
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, note_column(&screen), 6),
        AREA,
        &context(),
    );
    assert_eq!(screen.state.instances()[2].voicing_rotation, -1);
    assert_eq!(
        (0..4)
            .map(|lane| screen.state.resolved_note(LaneAddress::new(2, lane)))
            .collect::<Vec<_>>(),
        vec![Some(55), Some(60), Some(64), Some(67)]
    );
    assert_eq!(
        screen.state.instances()[2]
            .lanes
            .iter()
            .map(|lane| lane.base_note)
            .collect::<Vec<_>>(),
        before
    );

    // 高速wheel相当の連続downでもroot positionへwrapせず、そのまま下がり続ける。
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, note_column(&screen), 6),
        AREA,
        &context(),
    );
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, note_column(&screen), 6),
        AREA,
        &context(),
    );
    assert_eq!(screen.state.instances()[2].voicing_rotation, -3);
    assert_eq!(
        (0..4)
            .map(|lane| screen.state.resolved_note(LaneAddress::new(2, lane)))
            .collect::<Vec<_>>(),
        vec![Some(48), Some(52), Some(55), Some(60)]
    );

    screen.state.set_chord(
        ChordPlayback::new("D", "I".to_string(), vec![vec![62, 66, 69]]),
        Instant::now(),
    );
    assert_eq!(screen.state.instances()[2].voicing_rotation, -3);
    assert_eq!(
        (0..4)
            .map(|lane| screen.state.resolved_note(LaneAddress::new(2, lane)))
            .collect::<Vec<_>>(),
        vec![Some(50), Some(54), Some(57), Some(62)]
    );

    screen.state.set_chord(None, Instant::now());
    // chord OFF では chord 行が消えて行が1つ繰り上がる。行4が 4 voice の instance。
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollUp, note_column(&screen), 4),
        AREA,
        &context(),
    );
    assert_eq!(
        screen.state.instances()[2].lanes[0].base_note,
        before[0] + 1
    );
    assert_eq!(
        screen.state.instances()[2].lanes[1..]
            .iter()
            .map(|lane| lane.base_note)
            .collect::<Vec<_>>(),
        before[1..]
    );
}
