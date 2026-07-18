use super::*;

const OCTAVE: u8 = 12;

impl KeyboardState {
    // tキーで off → repeat → arp → off を循環する。和音未確定時はoffを維持する。
    pub(in crate::tui) fn cycle_note_playback(&mut self, now: Instant) -> Vec<[u8; 3]> {
        match self.note_playback_mode {
            NotePlaybackMode::Off => {
                if self.repeat_chord.is_empty() {
                    return Vec::new();
                }
                self.note_playback_mode = NotePlaybackMode::Repeat;
                self.periodic_next_at = Some(now + PERIODIC_INTERVAL);
                self.attack_repeat_chord()
            }
            NotePlaybackMode::Repeat => {
                self.note_playback_mode = NotePlaybackMode::Arp;
                let mut messages: Vec<[u8; 3]> =
                    self.repeat_sounding.drain(..).map(note_off).collect();
                messages.extend(self.restart_arp(now));
                messages
            }
            NotePlaybackMode::Arp => {
                self.note_playback_mode = NotePlaybackMode::Off;
                self.arp_next_index = 0;
                self.periodic_next_at = self
                    .periodic_digits_active()
                    .then(|| now + PERIODIC_INTERVAL);
                self.arp_sounding.take().map(note_off).into_iter().collect()
            }
        }
    }

    // patch切替後は先頭音から再開し、最初の音にも完全な250msを与える。
    pub(super) fn restart_arp(&mut self, now: Instant) -> Vec<[u8; 3]> {
        self.arp_next_index = 0;
        self.periodic_next_at = Some(now + PERIODIC_INTERVAL);
        self.attack_next_arp().into_iter().collect()
    }

    pub(super) fn advance_arp(&mut self) -> Vec<[u8; 3]> {
        let mut messages: Vec<[u8; 3]> =
            self.arp_sounding.take().map(note_off).into_iter().collect();
        if let Some(attack) = self.attack_next_arp() {
            messages.push(attack);
        } else {
            self.note_playback_mode = NotePlaybackMode::Off;
        }
        messages
    }

    fn attack_next_arp(&mut self) -> Option<[u8; 3]> {
        let sequence = self.arp_sequence();
        if sequence.is_empty() {
            return None;
        }
        let index = self.arp_next_index % sequence.len();
        let note = sequence[index];
        self.arp_next_index = (index + 1) % sequence.len();
        self.arp_sounding = Some(note);
        Some(note_on(note, self.velocity))
    }

    fn arp_sequence(&self) -> Vec<KeyboardNote> {
        let mut base = self.repeat_chord.clone();
        base.sort_unstable_by_key(|note| note.midi_note);
        let mut sequence = Vec::with_capacity(base.len() * 2);
        sequence.extend(base.iter().copied());
        sequence.extend(base.into_iter().filter_map(|note| {
            note.midi_note
                .checked_add(OCTAVE)
                .map(|midi_note| KeyboardNote { midi_note, ..note })
        }));
        sequence
    }
}

#[cfg(test)]
#[path = "arp_tests.rs"]
mod tests;
