//! 組み立て中の列の発音。lane ごとの owner を保って MIDI メッセージ列を作る。

use std::time::Instant;

use super::{
    measure_lane::TriggerTable, note_off, note_on, GridScheduledMessage, GridState, LaneAddress,
    SoundOwner, SoundingNote, CHORD_ROW, GRID_STEPS,
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

    /// 譜面の小節頭を待たずに、いま `address` の音を鳴らし始める。小節末で切れる。
    ///
    /// 1meas 伸ばすベースライン（`BassPattern::Whole`）は step 0 にしか Attack が無いので、
    /// 小節の途中で選ぶと次の小節頭まで最大1meas無音になり、wheel を回しても手応えが無い。
    /// 送った型をその場で聴けるよう、ここで1音だけ差し込む。
    ///
    /// `ahead` を [`GridState::silence_ahead`] に合わせるのは、先読みで既に送信済みの
    /// note off に後から追い越されて消されるのを防ぐため。
    pub fn preview_lane_now(
        &mut self,
        address: LaneAddress,
        now: Instant,
    ) -> Vec<GridScheduledMessage> {
        if !self.is_running() {
            return Vec::new();
        }
        let (Some(midi_note), Some(velocity_lane)) =
            (self.resolved_note(address), self.stored_lane_index(address))
        else {
            return Vec::new();
        };
        let ahead = self.silence_ahead(now);
        let timeline_seconds = self.silence_timeline_seconds();
        let instance_id = self.instance_id(address.instance);
        let owner = SoundOwner::Lane(address);

        // 同じ lane で鳴っている音は先に止める（`attack_current_step` と同じ手順）。
        let mut released = Vec::new();
        self.sounding.retain(|note| {
            if note.owner == owner {
                released.push((note.instance_id, note.midi_note));
                false
            } else {
                true
            }
        });
        let mut messages = released
            .into_iter()
            .map(|(instance_id, midi_note)| (instance_id, note_off(midi_note)))
            .collect::<Vec<_>>();

        // 組み立て済みの列の次から数えるので、ちょうど次の小節頭で切れる。そこで
        // 譜面の step 0 の Attack が鳴り、聴感上は途切れずに繋がる。
        let velocity = self.velocity.value_at(velocity_lane, self.schedule_index);
        self.sounding.push(SoundingNote {
            owner,
            instance_id,
            midi_note,
            remaining_steps: (GRID_STEPS - self.schedule_index) as u8,
        });
        messages.push((instance_id, note_on(midi_note, velocity)));

        messages
            .into_iter()
            .map(|(instance_id, message)| GridScheduledMessage {
                instance_id,
                ahead,
                timeline_seconds,
                message,
            })
            .collect()
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

#[cfg(test)]
mod tests;
