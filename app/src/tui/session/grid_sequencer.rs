//! Grid Sequencer のdomain sessionとhistory wire DTOの相互変換。

pub(super) fn grid_session_from_history(
    session: Option<crate::history::GridSequencerSessionState>,
) -> Option<super::super::grid_sequencer::GridSequencerSession> {
    let session = session.filter(|session| !session.instances.is_empty())?;
    let instances = session
        .instances
        .into_iter()
        .map(|instance| super::super::grid_sequencer::GridInstance {
            patch: instance.patch,
            lane_mode: match instance.lane_mode {
                crate::history::GridLaneModeState::Single => {
                    super::super::grid_sequencer::GridLaneMode::Single
                }
                crate::history::GridLaneModeState::BassOctave2 => {
                    super::super::grid_sequencer::GridLaneMode::BassOctave2
                }
                crate::history::GridLaneModeState::ChordVoices4 => {
                    super::super::grid_sequencer::GridLaneMode::ChordVoices4
                }
            },
            drum: instance.drum.map(history_drum_role_to_domain),
            voicing_rotation: instance.voicing_rotation,
            swing: super::super::grid_sequencer::clamp_swing(instance.swing),
            lanes: instance
                .lanes
                .into_iter()
                .map(|lane| super::super::grid_sequencer::GridLane {
                    base_note: lane.base_note,
                    pattern: super::super::grid_sequencer::NotePattern::from_steps(
                        lane.note_steps.into_iter().map(history_note_step_to_domain),
                    ),
                })
                .collect(),
        })
        .collect();
    let cycle_random = super::super::grid_sequencer::CycleRandom {
        patch: session.cycle_random.patch,
        note: session.cycle_random.note,
        drum: session.cycle_random.drum,
        arp: session.cycle_random.arp,
        chord: session.cycle_random.chord,
        bpm: session.cycle_random.bpm,
        swing: session.cycle_random.swing,
    };
    let fixed_chord = session
        .fixed_chord
        .map(|fixed| super::super::grid_sequencer::FixedChordProgression::new(fixed.input));
    Some(
        super::super::grid_sequencer::GridSequencerSession::new(instances, cycle_random)
            .with_fixed_chord(fixed_chord),
    )
}

fn history_drum_role_to_domain(
    role: crate::history::GridDrumRoleState,
) -> super::super::grid_sequencer::DrumRole {
    match role {
        crate::history::GridDrumRoleState::Kick => super::super::grid_sequencer::DrumRole::Kick,
        crate::history::GridDrumRoleState::Snare => super::super::grid_sequencer::DrumRole::Snare,
        crate::history::GridDrumRoleState::HiHat => super::super::grid_sequencer::DrumRole::HiHat,
        crate::history::GridDrumRoleState::Percussion => {
            super::super::grid_sequencer::DrumRole::Percussion
        }
    }
}

fn domain_drum_role_to_history(
    role: super::super::grid_sequencer::DrumRole,
) -> crate::history::GridDrumRoleState {
    match role {
        super::super::grid_sequencer::DrumRole::Kick => crate::history::GridDrumRoleState::Kick,
        super::super::grid_sequencer::DrumRole::Snare => crate::history::GridDrumRoleState::Snare,
        super::super::grid_sequencer::DrumRole::HiHat => crate::history::GridDrumRoleState::HiHat,
        super::super::grid_sequencer::DrumRole::Percussion => {
            crate::history::GridDrumRoleState::Percussion
        }
    }
}

fn history_note_step_to_domain(
    step: crate::history::GridNoteStepState,
) -> super::super::grid_sequencer::NoteStep {
    match step {
        crate::history::GridNoteStepState::Rest => super::super::grid_sequencer::NoteStep::Rest,
        crate::history::GridNoteStepState::Attack => super::super::grid_sequencer::NoteStep::Attack,
        crate::history::GridNoteStepState::Tie => super::super::grid_sequencer::NoteStep::Tie,
    }
}

fn domain_note_step_to_history(
    step: &super::super::grid_sequencer::NoteStep,
) -> crate::history::GridNoteStepState {
    match step {
        super::super::grid_sequencer::NoteStep::Rest => crate::history::GridNoteStepState::Rest,
        super::super::grid_sequencer::NoteStep::Attack => crate::history::GridNoteStepState::Attack,
        super::super::grid_sequencer::NoteStep::Tie => crate::history::GridNoteStepState::Tie,
    }
}

pub(super) fn grid_session_to_history(
    session: Option<super::super::grid_sequencer::GridSequencerSession>,
) -> Option<crate::history::GridSequencerSessionState> {
    session.map(|session| crate::history::GridSequencerSessionState {
        instances: session
            .instances
            .into_iter()
            .map(|instance| crate::history::GridSequencerInstanceState {
                patch: instance.patch,
                lane_mode: match instance.lane_mode {
                    super::super::grid_sequencer::GridLaneMode::Single => {
                        crate::history::GridLaneModeState::Single
                    }
                    super::super::grid_sequencer::GridLaneMode::BassOctave2 => {
                        crate::history::GridLaneModeState::BassOctave2
                    }
                    super::super::grid_sequencer::GridLaneMode::ChordVoices4 => {
                        crate::history::GridLaneModeState::ChordVoices4
                    }
                    // drum 行の lane の形は Single と同じ。役割は `drum` が持つ。
                    super::super::grid_sequencer::GridLaneMode::Drum => {
                        crate::history::GridLaneModeState::Single
                    }
                },
                drum: instance.drum.map(domain_drum_role_to_history),
                voicing_rotation: instance.voicing_rotation,
                swing: instance.swing,
                lanes: instance
                    .lanes
                    .into_iter()
                    .map(|lane| crate::history::GridSequencerLaneState {
                        base_note: lane.base_note,
                        note_steps: lane
                            .pattern
                            .steps()
                            .iter()
                            .map(domain_note_step_to_history)
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
        cycle_random: crate::history::GridCycleRandomState {
            patch: session.cycle_random.patch,
            note: session.cycle_random.note,
            drum: session.cycle_random.drum,
            arp: session.cycle_random.arp,
            chord: session.cycle_random.chord,
            bpm: session.cycle_random.bpm,
            swing: session.cycle_random.swing,
        },
        fixed_chord: session
            .fixed_chord
            .map(|fixed| crate::history::GridFixedChordState {
                input: fixed.input().to_string(),
            }),
    })
}

#[cfg(test)]
mod tests;
