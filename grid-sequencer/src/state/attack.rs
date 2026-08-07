//! 組み立て中の列の発音。lane ごとの owner を保って MIDI メッセージ列を作る。

use super::{
    measure_lane::TriggerTable, note_off, note_on, GridState, LaneAddress, SoundOwner,
    SoundingNote, CHORD_ROW, GRID_STEPS,
};

struct Attack {
    owner: SoundOwner,
    velocity_lane: usize,
    instance_id: u8,
    notes: Vec<u8>,
    steps: u8,
}

impl GridState {
    /// 組み立て中の列で note on が立っている lane を、instance/lane 順に発音する。
    pub(super) fn attack_current_step(&mut self) -> Vec<(u8, [u8; 3])> {
        let attacks = self.collect_attacks();
        let step = self.schedule_index;
        let mut messages = Vec::new();
        for attack in attacks {
            let velocity = self.velocity.value_at(attack.velocity_lane, step);

            // 同じlaneの再Attackだけを止める。同じinstanceの別laneは独立して伸ばす。
            let mut released = Vec::new();
            self.sounding.retain(|note| {
                if note.owner == attack.owner {
                    released.push((note.instance_id, note.midi_note));
                    false
                } else {
                    true
                }
            });
            messages.extend(
                released
                    .into_iter()
                    .map(|(instance_id, midi_note)| (instance_id, note_off(midi_note))),
            );
            for midi_note in attack.notes {
                self.sounding.push(SoundingNote {
                    owner: attack.owner,
                    instance_id: attack.instance_id,
                    midi_note,
                    remaining_steps: attack.steps,
                });
                messages.push((attack.instance_id, note_on(midi_note, velocity)));
            }
        }
        messages
    }

    /// stored lane × step の発音表。Velocity の抽選・表示に使う。
    pub(super) fn lane_trigger_table(&self) -> TriggerTable {
        let mut table = Vec::with_capacity(self.stored_lane_count());
        for (instance_index, instance) in self.instances.iter().enumerate() {
            for lane in 0..instance.lanes.len() {
                let address = LaneAddress::new(instance_index, lane);
                table.push(std::array::from_fn(|step| {
                    self.lane_triggers_at(address, step)
                }));
            }
        }
        table
    }

    /// instance × step の発音表。所属するactive laneのunionでCC1区間を決める。
    pub(super) fn instance_trigger_table(&self) -> TriggerTable {
        (0..self.instance_count())
            .map(|instance| {
                std::array::from_fn(|step| {
                    if self.chord.is_some() && instance == CHORD_ROW {
                        return step == 0
                            && self
                                .chord
                                .as_ref()
                                .is_some_and(|chord| !chord.current().is_empty());
                    }
                    self.instances[instance]
                        .lanes
                        .iter()
                        .enumerate()
                        .any(|(lane, _)| {
                            self.lane_triggers_at(LaneAddress::new(instance, lane), step)
                        })
                })
            })
            .collect()
    }

    fn collect_attacks(&self) -> Vec<Attack> {
        let step = self.schedule_index;
        let mut attacks = Vec::new();
        for (instance_index, instance) in self.instances.iter().enumerate() {
            let instance_id = self.instance_id(instance_index);
            if instance_index == CHORD_ROW {
                if let Some(notes) = self.chord.as_ref().map(|chord| chord.current()) {
                    if step == 0 && !notes.is_empty() {
                        attacks.push(Attack {
                            owner: SoundOwner::ChordInstance {
                                instance: instance_index,
                            },
                            velocity_lane: self
                                .stored_lane_index(LaneAddress::new(instance_index, 0))
                                .expect("instance has a primary lane"),
                            instance_id,
                            notes: notes.to_vec(),
                            steps: GRID_STEPS as u8,
                        });
                    }
                    continue;
                }
            }
            for (lane_index, lane) in instance.lanes.iter().enumerate() {
                let address = LaneAddress::new(instance_index, lane_index);
                let Some(note) = self.resolved_note(address) else {
                    continue;
                };
                if !lane.pattern.is_attack(step) {
                    continue;
                }
                attacks.push(Attack {
                    owner: SoundOwner::Lane(address),
                    velocity_lane: self
                        .stored_lane_index(address)
                        .expect("iterated lane has a flattened index"),
                    instance_id,
                    notes: vec![note],
                    steps: lane
                        .pattern
                        .attack_len(step)
                        .expect("an attack was just observed"),
                });
            }
        }
        attacks
    }

    fn lane_triggers_at(&self, address: LaneAddress, step: usize) -> bool {
        if self.chord.is_some() && address.instance == CHORD_ROW {
            return address.lane == 0
                && step == 0
                && self
                    .chord
                    .as_ref()
                    .is_some_and(|chord| !chord.current().is_empty());
        }
        self.resolved_note(address).is_some_and(|_| {
            self.lane(address)
                .is_some_and(|lane| lane.pattern.is_attack(step))
        })
    }

    pub(super) fn lane_sounding_end(&self, address: LaneAddress) -> Option<usize> {
        if self.chord.is_some() && address.instance == CHORD_ROW && address.lane == 0 {
            return self
                .chord
                .as_ref()
                .is_some_and(|chord| !chord.current().is_empty())
                .then_some(GRID_STEPS - 1);
        }
        self.resolved_note(address)?;
        self.lane(address)?.pattern.sounding_end()
    }

    pub(super) fn instance_sounding_end(&self, instance: usize) -> Option<usize> {
        if self.chord.is_some() && instance == CHORD_ROW {
            return self
                .chord
                .as_ref()
                .is_some_and(|chord| !chord.current().is_empty())
                .then_some(GRID_STEPS - 1);
        }
        self.instances
            .get(instance)?
            .lanes
            .iter()
            .enumerate()
            .fold(None, |end, (lane, _)| {
                end.max(self.lane_sounding_end(LaneAddress::new(instance, lane)))
            })
    }
}
