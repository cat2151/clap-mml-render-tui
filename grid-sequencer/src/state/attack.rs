//! 組み立て中の列の発音。行ごとのレーン値を載せて MIDI メッセージ列を作る。

use super::{
    cc1::control_change, measure_lane::TriggerTable, note_off, note_on, ChordPlayback, GridState,
    SoundingNote, StepDuration, CHORD_ROW,
};

impl GridState {
    /// 組み立て中の列で note on が立っている行を、CC1 を先頭にして発音する。
    pub(super) fn attack_current_step(&mut self) -> Vec<(u8, [u8; 3])> {
        let attacks = self.collect_attacks();
        let step = self.schedule_index;
        let mut messages = Vec::new();
        for (row, instance_id, notes, steps) in attacks {
            messages.push((instance_id, control_change(self.cc1.value_at(row, step))));
            // 和音は構成音すべてを同じ velocity で鳴らす。CC1 を1件だけ送るのと同じ扱い。
            let velocity = self.velocity.value_at(row, step);

            // 同じ instance で鳴っている音はすべて止めてから鳴らし直す。1音とは
            // 限らない（chord mode の和音）ので、1件だけ差し替えてはいけない。
            let mut released = Vec::new();
            self.sounding.retain(|note| {
                if note.instance_id == instance_id {
                    released.push(note.midi_note);
                    false
                } else {
                    true
                }
            });
            messages.extend(
                released
                    .into_iter()
                    .map(|midi_note| (instance_id, note_off(midi_note))),
            );
            for midi_note in notes {
                self.sounding.push(SoundingNote {
                    instance_id,
                    midi_note,
                    remaining_steps: steps,
                });
                messages.push((instance_id, note_on(midi_note, velocity)));
            }
        }
        messages
    }

    /// 行×ステップの発音表。レーンの送信対象はこれを見て決める。
    pub(super) fn trigger_table(&self) -> TriggerTable {
        (0..self.rows.len())
            .map(|row| std::array::from_fn(|step| self.row_triggers_at(row, step)))
            .collect()
    }

    /// この列で鳴らす (row, instance, note number 群, 持続ステップ数) を行順に集める。
    fn collect_attacks(&self) -> Vec<(usize, u8, Vec<u8>, u8)> {
        let step = self.schedule_index;
        let chord = self.chord.as_ref().map(ChordPlayback::current);
        let mut attacks = Vec::new();
        for (index, row) in self.rows.iter().enumerate() {
            let instance_id = self.instance_id(index);
            match chord {
                // chord 行は小節頭に CC1 を1件送り、その後で全構成音を鳴らす。
                Some(notes) if index == CHORD_ROW => {
                    if step == 0 && !notes.is_empty() {
                        attacks.push((
                            index,
                            instance_id,
                            notes.to_vec(),
                            StepDuration::Whole.steps(),
                        ));
                    }
                }
                _ if row.cells[step] => {
                    attacks.push((index, instance_id, vec![row.note], row.duration.steps()))
                }
                _ => {}
            }
        }
        attacks
    }

    fn row_triggers_at(&self, row: usize, step: usize) -> bool {
        match self.chord.as_ref() {
            Some(chord) if row == CHORD_ROW => step == 0 && !chord.current().is_empty(),
            _ => self.rows[row].cells[step],
        }
    }
}
