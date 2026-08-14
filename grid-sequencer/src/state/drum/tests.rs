use cmrt_rhythm::{DrumPattern, DrumRole, HatPattern, KickPattern, PercPattern, SnarePattern};

use crate::{
    state::{GridInstance, GridLaneMode},
    ChordPlayback, GridState, LaneAddress, NoteStep, FIRST_DRUM_ROW, FULL_DRUM_TRACK_COUNT,
    GRID_STEPS,
};

use super::{apply_drum_roles, drum_role_for};

fn roles(track_count: usize) -> Vec<Option<DrumRole>> {
    let state = GridState::with_instance_count(track_count);
    state
        .instances()
        .iter()
        .map(|instance| instance.drum)
        .collect()
}

/// 画面は instance 昇順に上から描かれるので、下から kick・snare・hi-hat・percussion。
#[test]
fn seven_tracks_fill_every_role_with_kick_at_the_bottom() {
    assert_eq!(
        roles(FULL_DRUM_TRACK_COUNT),
        [
            None,
            None,
            None,
            Some(DrumRole::Percussion),
            Some(DrumRole::HiHat),
            Some(DrumRole::Snare),
            Some(DrumRole::Kick),
        ]
    );
}

/// 4 role を使い切ったあとの行は従来どおり自由。
#[test]
fn extra_rows_beyond_the_four_roles_stay_free() {
    let eight = roles(8);
    assert_eq!(eight[..FULL_DRUM_TRACK_COUNT], roles(FULL_DRUM_TRACK_COUNT));
    assert_eq!(eight[FULL_DRUM_TRACK_COUNT], None);
}

#[test]
fn fewer_than_four_tracks_have_no_drum_row() {
    for track_count in 1..=3 {
        assert!(
            roles(track_count).iter().all(Option::is_none),
            "{track_count}"
        );
    }
}

/// track 4 は drum 行が1つだけ。役割は抽選だが、4 role のどれかにはなる。
#[test]
fn four_tracks_have_one_drum_row_with_a_drawn_role() {
    let roles = roles(4);
    assert_eq!(roles[..FIRST_DRUM_ROW], [None, None, None]);
    let role = roles[FIRST_DRUM_ROW].expect("the fourth row is a drum row");
    assert!(DrumRole::ALL.contains(&role));
}

/// 抽選した役割は保存値として固定される。入り直しのたびに変わると patch も譜面も揺れる。
#[test]
fn a_drawn_role_is_kept_across_reassignment() {
    let mut instances = (0..4).map(GridInstance::new).collect::<Vec<_>>();
    apply_drum_roles(&mut instances);
    let drawn = instances[FIRST_DRUM_ROW].drum;
    assert!(drawn.is_some());
    for _ in 0..16 {
        apply_drum_roles(&mut instances);
        assert_eq!(instances[FIRST_DRUM_ROW].drum, drawn);
    }
}

#[test]
fn drum_rows_get_the_drum_lane_mode_and_a_single_lane() {
    let state = GridState::with_instance_count(FULL_DRUM_TRACK_COUNT);
    for instance in &state.instances()[FIRST_DRUM_ROW..] {
        assert_eq!(instance.lane_mode, GridLaneMode::Drum);
        assert_eq!(instance.lanes.len(), 1);
    }
}

/// 役割が付いた瞬間に譜面が入る。空のままだと drum 行だけ無音で始まる。
#[test]
fn assigning_a_role_writes_its_rhythm() {
    let state = GridState::with_instance_count(FULL_DRUM_TRACK_COUNT);
    let kick = &state.instances()[FULL_DRUM_TRACK_COUNT - 1];
    assert_eq!(kick.drum, Some(DrumRole::Kick));
    let attacks = (0..GRID_STEPS)
        .filter(|step| kick.lanes[0].pattern.is_attack(*step))
        .collect::<Vec<_>>();
    assert!(
        attacks == [0, 4, 8, 12] || attacks == [0, 10],
        "{attacks:?}"
    );
}

#[test]
fn one_and_three_offbeat_kick_matches_the_requested_grid_pattern() {
    let mut state = GridState::with_instance_count(FULL_DRUM_TRACK_COUNT);
    let row = FULL_DRUM_TRACK_COUNT - 1;

    let _ = state.apply_drum_pattern(row, DrumPattern::Kick(KickPattern::OneAndThreeOffbeat));

    assert_eq!(pattern_text(&state.instances()[row]), "#---------#-----");
}

/// 「次の音まで伸ばしっぱなし」が譜面（Attack + Tie）としてそのまま出ること。
#[test]
fn a_written_rhythm_ties_until_the_next_attack() {
    let mut state = GridState::with_instance_count(FULL_DRUM_TRACK_COUNT);
    let row = FULL_DRUM_TRACK_COUNT - 2;
    let _ = state.apply_drum_pattern(row, DrumPattern::Snare(SnarePattern::Backbeat));
    let pattern = &state.instances()[row].lanes[0].pattern;
    assert_eq!(pattern.step(4), Some(NoteStep::Attack));
    assert!((5..12).all(|step| pattern.step(step) == Some(NoteStep::Tie)));
    assert_eq!(pattern.step(12), Some(NoteStep::Attack));
    assert!((13..GRID_STEPS).all(|step| pattern.step(step) == Some(NoteStep::Tie)));
}

#[test]
fn offbeat_quarter_hat_matches_the_documented_grid_pattern() {
    let mut state = GridState::with_instance_count(FULL_DRUM_TRACK_COUNT);
    let row = FULL_DRUM_TRACK_COUNT - 3;
    assert_eq!(state.drum_role(row), Some(DrumRole::HiHat));

    let _ = state.apply_drum_pattern(row, DrumPattern::Hat(HatPattern::OffbeatQuarter));

    assert_eq!(pattern_text(&state.instances()[row]), "..#---#---#---#-");
}

#[test]
fn random_percussion_writes_one_to_three_held_notes() {
    let mut state = GridState::with_instance_count(FULL_DRUM_TRACK_COUNT);
    let row = FIRST_DRUM_ROW;
    let _ = state.apply_drum_pattern(row, DrumPattern::Perc(PercPattern::Random));
    let pattern = &state.instances()[row].lanes[0].pattern;
    let attacks = (0..GRID_STEPS)
        .filter(|step| pattern.is_attack(*step))
        .collect::<Vec<_>>();

    assert!((1..=3).contains(&attacks.len()), "{attacks:?}");
    for (index, attack) in attacks.iter().enumerate() {
        let expected = attacks.get(index + 1).copied().unwrap_or(GRID_STEPS) - attack;
        assert_eq!(pattern.attack_len(*attack), Some(expected as u8));
    }
}

/// 役割の違う型は当たらない。wheel が別の行の list を当てないための番人。
#[test]
fn a_pattern_from_another_role_is_rejected() {
    let mut state = GridState::with_instance_count(FULL_DRUM_TRACK_COUNT);
    let kick_row = FULL_DRUM_TRACK_COUNT - 1;
    assert!(!state.apply_drum_pattern(kick_row, DrumPattern::Hat(HatPattern::Sixteenth)));
    assert!(!state.apply_drum_pattern(0, DrumPattern::Kick(KickPattern::Quarter)));
}

/// chord mode 中でも drum の音高は動かない。動くと kick のピッチが小節ごとに変わる。
#[test]
fn chord_mode_does_not_snap_the_drum_pitch() {
    let mut state = GridState::with_instance_count(FULL_DRUM_TRACK_COUNT);
    let row = FULL_DRUM_TRACK_COUNT - 1;
    let before = state.instances()[row].lanes[0].base_note;
    let chord = ChordPlayback::new("C", "I".to_string(), vec![vec![61, 65, 68]])
        .expect("a non-empty progression");
    let _ = state.set_chord(Some(chord), std::time::Instant::now());

    assert_eq!(
        state.resolved_note(LaneAddress::new(row, 0)),
        Some(before),
        "drum row must ignore the chord"
    );
}

#[test]
fn a_free_row_is_never_given_a_role() {
    let mut rng = rand::rng();
    for index in 0..FIRST_DRUM_ROW {
        assert_eq!(
            drum_role_for(FULL_DRUM_TRACK_COUNT, index, None, &mut rng),
            None
        );
    }
    assert_eq!(drum_role_for(16, 15, None, &mut rng), None);
}

/// 無差別の引き直しでも drum 行は drum のまま。音高が動くと打楽器の音色が変わり、
/// 汎用の譜面が入るとリズムでなくなる。
#[test]
fn randomizing_keeps_the_drum_rows_on_a_drum_rhythm() {
    let mut state = GridState::with_instance_count(FULL_DRUM_TRACK_COUNT);
    let kick_row = FULL_DRUM_TRACK_COUNT - 1;
    let note = state.instances()[kick_row].lanes[0].base_note;

    for _ in 0..16 {
        let patterns = state.draw_pattern_combination(crate::CycleRandom::ALL, false);
        crate::randomize_instance_slice(
            state.instances_mut(),
            &[],
            crate::CycleRandom::ALL,
            None,
            patterns,
        );
        let kick = &state.instances()[kick_row];
        assert_eq!(kick.lanes[0].base_note, note, "drum の音高は動かさない");
        assert!(
            DrumPattern::all_for(DrumRole::Kick)
                .iter()
                .any(|pattern| written(*pattern) == pattern_text(kick)),
            "{}",
            pattern_text(kick)
        );
    }
}

/// 役割ごとの型を書いたときの譜面。
fn written(pattern: DrumPattern) -> String {
    let mut instance = GridInstance::new(FULL_DRUM_TRACK_COUNT - 1);
    instance.set_drum_role(FULL_DRUM_TRACK_COUNT - 1, Some(pattern.role()));
    super::write_drum_pattern(&mut instance, pattern, &mut rand::rng());
    pattern_text(&instance)
}

fn pattern_text(instance: &GridInstance) -> String {
    instance.lanes[0]
        .pattern
        .steps()
        .iter()
        .map(|step| match step {
            NoteStep::Rest => '.',
            NoteStep::Attack => '#',
            NoteStep::Tie => '-',
        })
        .collect()
}
