use super::*;

const OCTAVE: u8 = 12;

impl KeyboardState {
    pub(in crate::tui) fn replace_repeat_chords(
        &mut self,
        progression: Vec<Vec<u8>>,
        now: Instant,
        restart_now: bool,
    ) -> Vec<[u8; 3]> {
        self.repeat_chords = progression
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
            .collect();
        self.reset_progression_position();
        self.repeat_elapsed_ticks = 0;

        let mut messages: Vec<[u8; 3]> = self
            .repeat_sounding
            .drain(..)
            .map(|note| note_off(note.midi_note))
            .collect();
        messages.extend(
            self.arp_sounding
                .take()
                .map(|note| note_off(note.midi_note)),
        );

        if self.note_playback_mode == NotePlaybackMode::Off {
            return messages;
        }
        if !restart_now {
            self.refresh_pending = true;
            return messages;
        }
        if self.note_playback_uses_arp() {
            messages.extend(self.restart_arp(now));
        } else {
            self.periodic_next_at = Some(now + PERIODIC_INTERVAL);
            messages.extend(self.attack_repeat_chord());
        }
        messages
    }

    // tキーで off → repeat → arp → auto → off を循環する。和音未確定時はoffを維持する。
    pub(in crate::tui) fn cycle_note_playback(&mut self, now: Instant) -> Vec<[u8; 3]> {
        match self.note_playback_mode {
            NotePlaybackMode::Off => {
                if self.repeat_chords.is_empty() {
                    return Vec::new();
                }
                self.note_playback_mode = NotePlaybackMode::Repeat;
                self.reset_progression_position();
                self.repeat_elapsed_ticks = 0;
                self.periodic_next_at = Some(now + PERIODIC_INTERVAL);
                self.attack_repeat_chord()
            }
            NotePlaybackMode::Repeat => {
                self.note_playback_mode = NotePlaybackMode::Arp;
                self.repeat_elapsed_ticks = 0;
                let mut messages: Vec<[u8; 3]> = self
                    .repeat_sounding
                    .drain(..)
                    .map(|note| note_off(note.midi_note))
                    .collect();
                messages.extend(self.restart_arp(now));
                messages
            }
            NotePlaybackMode::Arp => {
                self.note_playback_mode = NotePlaybackMode::Auto;
                if self.note_playback_uses_arp() {
                    self.repeat_elapsed_ticks = 0;
                    Vec::new()
                } else {
                    self.repeat_elapsed_ticks = 0;
                    let mut messages: Vec<[u8; 3]> = self
                        .arp_sounding
                        .take()
                        .map(|note| note_off(note.midi_note))
                        .into_iter()
                        .collect();
                    messages.extend(self.attack_repeat_chord());
                    messages
                }
            }
            NotePlaybackMode::Auto => {
                self.note_playback_mode = NotePlaybackMode::Off;
                self.reset_progression_position();
                self.repeat_elapsed_ticks = 0;
                self.periodic_next_at = self
                    .periodic_digits_active()
                    .then(|| now + PERIODIC_INTERVAL);
                let mut messages: Vec<[u8; 3]> = self
                    .repeat_sounding
                    .drain(..)
                    .map(|note| note_off(note.midi_note))
                    .collect();
                messages.extend(
                    self.arp_sounding
                        .take()
                        .map(|note| note_off(note.midi_note)),
                );
                messages
            }
        }
    }

    // patch切替後は先頭音から再開し、最初の音にも完全な250msを与える。
    pub(super) fn restart_arp(&mut self, now: Instant) -> Vec<[u8; 3]> {
        self.reset_progression_position();
        self.repeat_elapsed_ticks = 0;
        self.periodic_next_at = Some(now + PERIODIC_INTERVAL);
        self.attack_next_arp().into_iter().collect()
    }

    pub(super) fn advance_arp(&mut self) -> Vec<[u8; 3]> {
        let mut messages: Vec<[u8; 3]> = self
            .arp_sounding
            .take()
            .map(|note| note_off(note.midi_note))
            .into_iter()
            .collect();
        if let Some(attack) = self.attack_next_arp() {
            messages.push(attack);
        } else {
            self.note_playback_mode = NotePlaybackMode::Off;
            self.repeat_elapsed_ticks = 0;
        }
        messages
    }

    fn attack_next_arp(&mut self) -> Option<[u8; 3]> {
        let mut sequence = self.arp_sequence();
        if sequence.is_empty() {
            return None;
        }
        if self.arp_next_index >= sequence.len() {
            self.advance_repeat_chord();
            self.arp_next_index = 0;
            sequence = self.arp_sequence();
        }
        let note = sequence[self.arp_next_index];
        self.arp_next_index += 1;
        self.arp_sounding = Some(note);
        Some(note_on(note.midi_note, self.velocity))
    }

    fn arp_sequence(&self) -> Vec<PlaybackNote> {
        let mut base = self.current_repeat_chord().to_vec();
        base.sort_unstable_by_key(|note| note.midi_note);
        let mut sequence = Vec::with_capacity(base.len() * 2);
        sequence.extend(base.iter().copied());
        sequence.extend(base.into_iter().filter_map(|note| {
            note.midi_note
                .checked_add(OCTAVE)
                .map(|midi_note| PlaybackNote { midi_note })
        }));
        sequence
    }

    pub(super) fn current_repeat_chord(&self) -> &[PlaybackNote] {
        self.repeat_chords
            .get(self.repeat_chord_index)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(super) fn advance_repeat_chord(&mut self) {
        if !self.repeat_chords.is_empty() {
            self.repeat_chord_index = (self.repeat_chord_index + 1) % self.repeat_chords.len();
        }
    }

    pub(super) fn reset_progression_position(&mut self) {
        self.repeat_chord_index = 0;
        self.arp_next_index = 0;
    }
}

#[cfg(test)]
#[path = "arp_tests.rs"]
mod tests;
