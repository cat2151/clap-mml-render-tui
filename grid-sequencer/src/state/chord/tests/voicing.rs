use super::*;

#[test]
fn same_step_chord_voice_attacks_remain_four_independent_note_ons() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(2);
    // Schedulingはvoicingを解釈しない。mono patchでも4 eventをそのままpluginへ渡す。
    state.instances[1].patch = Some("Bass/Mono.fxp".to_string());
    for lane in &mut state.instances[1].lanes {
        lane.pattern.draw_span(0, 0);
    }
    let seventh = ChordPlayback::new("C", "I7".to_string(), vec![vec![60, 64, 67, 71]]).unwrap();
    state.set_chord(Some(seventh), now);
    state.start(now);
    let notes = step_at(&mut state, now)
        .into_iter()
        .filter(|message| message.instance_id == 1 && message.message[0] == 0x90)
        .map(|message| message.message[1])
        .collect::<Vec<_>>();
    assert_eq!(notes, vec![60, 64, 67, 71]);
}

#[test]
fn signed_voicing_rotation_moves_the_new_bottom_note_in_wheel_direction() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(2);
    let seventh = ChordPlayback::new("C", "I7".to_string(), vec![vec![60, 64, 67, 71]]).unwrap();
    state.set_chord(Some(seventh), now);

    assert_eq!(
        voices4(&state),
        vec![Some(60), Some(64), Some(67), Some(71)]
    );

    state.instances[1].voicing_rotation = 1;
    assert_eq!(
        voices4(&state),
        vec![Some(64), Some(67), Some(71), Some(72)]
    );
    state.instances[1].voicing_rotation = 3;
    assert_eq!(
        voices4(&state),
        vec![Some(71), Some(72), Some(76), Some(79)]
    );
    state.instances[1].voicing_rotation = -1;
    assert_eq!(
        voices4(&state),
        vec![Some(59), Some(60), Some(64), Some(67)]
    );
    state.instances[1].voicing_rotation = -4;
    assert_eq!(
        voices4(&state),
        vec![Some(48), Some(52), Some(55), Some(59)]
    );
}

#[test]
fn downward_voicing_rotation_survives_a_chord_change() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(2);
    state.set_chord(
        ChordPlayback::new(
            "C",
            "I-II".to_string(),
            vec![vec![60, 64, 67], vec![62, 65, 69]],
        ),
        now,
    );
    for _ in 0..3 {
        assert!(state.rotate_chord_voicing(1, crate::PitchDirection::Down));
    }
    assert_eq!(state.instances[1].voicing_rotation, -3);
    assert_eq!(
        voices4(&state),
        vec![Some(48), Some(52), Some(55), Some(60)]
    );

    state.advance_chord();

    assert_eq!(state.instances[1].voicing_rotation, -3);
    assert_eq!(
        voices4(&state),
        vec![Some(50), Some(53), Some(57), Some(62)]
    );

    let held_instances = state.instances().to_vec();
    state.stage_next_cycle_in_place(
        held_instances,
        ChordPlayback::new("F", "I".to_string(), vec![vec![65, 69, 72]]).unwrap(),
    );
    state.advance_chord();

    assert_eq!(state.instances[1].voicing_rotation, -3);
    assert_eq!(
        voices4(&state),
        vec![Some(53), Some(57), Some(60), Some(65)]
    );
}

fn voices4(state: &GridState) -> Vec<Option<u8>> {
    (0..4)
        .map(|lane| state.resolved_note(LaneAddress::new(1, lane)))
        .collect()
}

#[test]
fn triad_schedules_the_bottom_voice_one_octave_up_on_the_fourth_lane() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(2);
    for lane in &mut state.instances[1].lanes {
        lane.pattern.draw_span(0, 0);
    }
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    let notes = step_at(&mut state, now)
        .into_iter()
        .filter(|message| message.instance_id == 1 && message.message[0] == 0x90)
        .map(|message| message.message[1])
        .collect::<Vec<_>>();
    assert_eq!(notes, vec![60, 64, 67, 72]);
}

#[test]
fn reset_releases_every_sounding_voice_of_the_shared_instance() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(2);
    for lane in &mut state.instances[1].lanes {
        lane.pattern.draw_span(0, 3);
    }
    state.set_chord(
        ChordPlayback::new("C", "I7".to_string(), vec![vec![60, 64, 67, 71]]),
        now,
    );
    state.start(now);
    step_at(&mut state, now);

    let offs = state
        .take_reset_messages()
        .into_iter()
        .filter(|message| message.instance_id == 1)
        .map(|message| message.message)
        .collect::<Vec<_>>();
    assert_eq!(
        offs,
        vec![[0x80, 60, 0], [0x80, 64, 0], [0x80, 67, 0], [0x80, 71, 0],]
    );
}
