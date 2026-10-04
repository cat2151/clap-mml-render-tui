use super::*;

pub(super) struct ScheduledProgression {
    at: Instant,
    chords: Vec<Vec<PlaybackNote>>,
}

impl KeyboardState {
    pub(crate) fn discard_scheduled_progression(&mut self) -> bool {
        self.scheduled_progression.take().is_some()
    }

    pub(crate) fn restart_committed_progression(
        &mut self,
        now: Instant,
        ready: bool,
    ) -> Vec<[u8; 3]> {
        let chords = self
            .repeat_chords
            .iter()
            .map(|chord| chord.iter().map(|note| note.midi_note).collect())
            .collect();
        self.replace_repeat_chords(chords, now, ready)
    }

    /// 実音時刻に対応する進行。先読み中は旧進行の描画位置を旧進行に適用する。
    pub fn repeat_chords_at(&self, now: Instant) -> &[Vec<PlaybackNote>] {
        match &self.scheduled_progression {
            Some(progression) if progression.at <= now => &progression.chords,
            _ => &self.repeat_chords,
        }
    }

    pub(crate) fn apply_scheduled_progression(&mut self, now: Instant) {
        if self
            .scheduled_progression
            .as_ref()
            .is_some_and(|progression| progression.at <= now)
        {
            self.repeat_chords = self.scheduled_progression.take().unwrap().chords;
        }
    }

    pub(super) fn playback_progression(&self) -> &[Vec<PlaybackNote>] {
        self.scheduled_progression
            .as_ref()
            .map_or(&self.repeat_chords, |progression| &progression.chords)
    }

    pub(super) fn advance_progression(
        &mut self,
        at: Instant,
        next: &mut impl FnMut(Instant) -> Option<Vec<Vec<u8>>>,
    ) {
        let len = self.playback_progression().len();
        if len == 0 {
            return;
        }
        if self.repeat_chord_index + 1 == len {
            if let Some(chords) = next(at) {
                let chords = playback_chords(chords);
                if !chords.is_empty() {
                    self.scheduled_progression = Some(ScheduledProgression { at, chords });
                }
            }
            self.reset_progression_position();
        } else {
            self.repeat_chord_index += 1;
        }
    }
}

pub(super) fn playback_chords(progression: Vec<Vec<u8>>) -> Vec<Vec<PlaybackNote>> {
    progression
        .into_iter()
        .filter_map(|midi_notes| {
            let mut seen = [false; 128];
            let chord = midi_notes
                .into_iter()
                .filter(|&note| {
                    let is_new = !seen[usize::from(note)];
                    seen[usize::from(note)] = true;
                    is_new
                })
                .map(|midi_note| PlaybackNote { midi_note })
                .collect::<Vec<_>>();
            (!chord.is_empty()).then_some(chord)
        })
        .collect()
}
