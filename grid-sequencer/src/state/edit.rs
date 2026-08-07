//! 人間が grid を編集するときの domain API。

use super::{
    chord::rotated_chord_voice, GridLaneMode, GridState, LaneAddress, NotePattern, CHORD_ROW,
    GRID_STEPS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PitchDirection {
    Up,
    Down,
}

impl GridState {
    /// instance が共有する patch を差し替える。
    pub fn set_instance_patch(&mut self, instance: usize, patch: String) -> bool {
        let Some(item) = self.instances.get_mut(instance) else {
            return false;
        };
        if item.patch.as_deref() == Some(patch.as_str()) {
            return false;
        }
        item.patch = Some(patch);
        true
    }

    pub fn draw_note_span(&mut self, address: LaneAddress, start: usize, end: usize) -> bool {
        if !self.note_pattern_is_editable(address) || start >= GRID_STEPS {
            return false;
        }
        if !self
            .lane_mut(address)
            .is_some_and(|lane| lane.pattern.draw_span(start, end))
        {
            return false;
        }
        self.refresh_lane_display_patterns();
        true
    }

    pub fn erase_note_at(&mut self, address: LaneAddress, step: usize) -> bool {
        if !self.note_pattern_is_editable(address) || step >= GRID_STEPS {
            return false;
        }
        if !self
            .lane_mut(address)
            .is_some_and(|lane| lane.pattern.erase_note_at(step))
        {
            return false;
        }
        self.refresh_lane_display_patterns();
        true
    }

    pub fn restore_note_pattern(&mut self, address: LaneAddress, pattern: NotePattern) -> bool {
        if !self.note_pattern_is_editable(address) {
            return false;
        }
        let Some(lane) = self.lane_mut(address) else {
            return false;
        };
        if lane.pattern == pattern {
            return false;
        }
        lane.pattern = pattern;
        self.refresh_lane_display_patterns();
        true
    }

    pub fn clear_notes(&mut self) -> bool {
        let chord_on = self.chord.is_some();
        let mut changed = false;
        for (instance_index, instance) in self.instances.iter_mut().enumerate() {
            if chord_on && instance_index == CHORD_ROW {
                continue;
            }
            for lane in &mut instance.lanes {
                changed |= lane.pattern.clear();
            }
        }
        if changed {
            self.refresh_lane_display_patterns();
        }
        changed
    }

    pub fn move_lane_pitch(&mut self, address: LaneAddress, direction: PitchDirection) -> bool {
        if !self.lane_parameter_is_editable(address) {
            return false;
        }
        let lane = self.lane(address).expect("editable lane exists");
        let next = match self.chord.as_ref() {
            Some(chord) => next_chord_tone(
                self.resolved_note(address).unwrap_or(lane.base_note),
                direction,
                &chord.pitch_classes(),
            ),
            None => match direction {
                PitchDirection::Up => lane.base_note.checked_add(1).filter(|note| *note <= 127),
                PitchDirection::Down => lane.base_note.checked_sub(1),
            },
        };
        self.set_lane_base_note(address, next)
    }

    pub fn move_lane_octave(&mut self, address: LaneAddress, direction: PitchDirection) -> bool {
        if !self.lane_parameter_is_editable(address) {
            return false;
        }
        let lane = self.lane(address).expect("editable lane exists");
        let anchor = self.resolved_note(address).unwrap_or(lane.base_note);
        let next = match direction {
            PitchDirection::Up => anchor.checked_add(12).filter(|note| *note <= 127),
            PitchDirection::Down => anchor.checked_sub(12),
        };
        self.set_lane_base_note(address, next)
    }

    /// ChordVoices4を1構成音ぶん上下へ転回する。どのvoiceのNOTE欄から操作してもinstance単位。
    pub fn rotate_chord_voicing(
        &mut self,
        instance_index: usize,
        direction: PitchDirection,
    ) -> bool {
        let notes = self
            .chord
            .as_ref()
            .map(|chord| {
                let count = chord.current().len().min(super::CHORD_VOICE_LANES);
                chord.current()[..count].to_vec()
            })
            .unwrap_or_default();
        let voice_count = notes.len();
        let Some(instance) = self.instances.get_mut(instance_index) else {
            return false;
        };
        if instance_index == CHORD_ROW
            || instance.lane_mode != GridLaneMode::ChordVoices4
            || voice_count < 2
        {
            return false;
        }
        let next = match direction {
            PitchDirection::Up => instance.voicing_rotation.checked_add(1),
            PitchDirection::Down => instance.voicing_rotation.checked_sub(1),
        };
        let Some(next) = next else {
            return false;
        };
        // 構成音数でwrapしない。3回downなら同じroot positionの1octave下になる。
        // MIDI note numberの範囲を出る直前でだけ止める。
        let last_lane = if voice_count == 3 {
            super::CHORD_VOICE_LANES - 1
        } else {
            voice_count - 1
        };
        if rotated_chord_voice(&notes, last_lane, next).is_none() {
            return false;
        }
        instance.voicing_rotation = next;
        true
    }

    fn lane_parameter_is_editable(&self, address: LaneAddress) -> bool {
        let Some(instance) = self.instances.get(address.instance) else {
            return false;
        };
        if instance.lanes.get(address.lane).is_none()
            || (self.chord.is_some() && address.instance == CHORD_ROW)
        {
            return false;
        }
        // chord voiceの音高は現在の構成音から自動導出する。
        !(self.chord.is_some() && instance.lane_mode == GridLaneMode::ChordVoices4)
    }

    fn note_pattern_is_editable(&self, address: LaneAddress) -> bool {
        self.lane(address).is_some() && !(self.chord.is_some() && address.instance == CHORD_ROW)
    }

    fn set_lane_base_note(&mut self, address: LaneAddress, note: Option<u8>) -> bool {
        let Some(note) = note else {
            return false;
        };
        let Some(lane) = self.lane_mut(address) else {
            return false;
        };
        if lane.base_note == note {
            return false;
        }
        lane.base_note = note;
        true
    }
}

fn next_chord_tone(
    current: u8,
    direction: PitchDirection,
    pitch_classes: &[bool; 12],
) -> Option<u8> {
    match direction {
        PitchDirection::Up => ((usize::from(current) + 1)..=127)
            .find(|note| pitch_classes[note % 12])
            .map(|note| note as u8),
        PitchDirection::Down => (0..usize::from(current))
            .rev()
            .find(|note| pitch_classes[note % 12])
            .map(|note| note as u8),
    }
}

#[cfg(test)]
mod tests;
