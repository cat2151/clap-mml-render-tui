use super::*;

const OCTAVE: u8 = 12;

impl KeyboardState {
    pub fn replace_repeat_chords(
        &mut self,
        progression: Vec<Vec<u8>>,
        now: Instant,
        restart_now: bool,
    ) -> Vec<[u8; 3]> {
        self.repeat_chords = progression::playback_chords(progression);
        self.scheduled_progression = None;
        self.reset_progression_position();
        self.repeat_elapsed_ticks = 0;
        self.sounding.clear();

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
            self.restart_periodic_clock(now);
            messages.extend(self.attack_repeat_chord(now));
        }
        messages
    }

    // tキーで off → auto → repeat → arp → off を循環する。和音未確定時はoffを維持する。
    pub fn cycle_note_playback(&mut self, now: Instant) -> Vec<[u8; 3]> {
        match self.note_playback_mode {
            NotePlaybackMode::Off => {
                if self.repeat_chords.is_empty() {
                    return Vec::new();
                }
                self.note_playback_mode = NotePlaybackMode::Auto;
                if self.note_playback_uses_arp() {
                    return self.restart_arp(now);
                }
                self.reset_progression_position();
                self.repeat_elapsed_ticks = 0;
                self.restart_periodic_clock(now);
                self.attack_repeat_chord(now)
            }
            NotePlaybackMode::Auto => {
                let was_arp = self.note_playback_uses_arp();
                self.note_playback_mode = NotePlaybackMode::Repeat;
                if !was_arp {
                    return Vec::new();
                }
                self.repeat_elapsed_ticks = 0;
                let mut messages: Vec<[u8; 3]> = self
                    .arp_sounding
                    .take()
                    .map(|note| note_off(note.midi_note))
                    .into_iter()
                    .collect();
                messages.extend(self.attack_repeat_chord(now));
                messages
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
                self.note_playback_mode = NotePlaybackMode::Off;
                self.reset_progression_position();
                self.repeat_elapsed_ticks = 0;
                self.sounding.clear();
                if self.periodic_digits_active() {
                    self.restart_periodic_clock(now);
                } else {
                    self.stop_periodic_clock();
                }
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
        self.restart_periodic_clock(now);
        self.attack_next_arp(now, &mut |_| None)
            .into_iter()
            .collect()
    }

    pub(super) fn advance_arp(
        &mut self,
        at: Instant,
        next: &mut impl FnMut(Instant) -> Option<Vec<Vec<u8>>>,
    ) -> Vec<[u8; 3]> {
        let mut messages: Vec<[u8; 3]> = self
            .arp_sounding
            .take()
            .map(|note| note_off(note.midi_note))
            .into_iter()
            .collect();
        if let Some(attack) = self.attack_next_arp(at, next) {
            messages.push(attack);
        } else {
            self.sounding.clear();
            self.note_playback_mode = NotePlaybackMode::Off;
            self.repeat_elapsed_ticks = 0;
        }
        messages
    }

    fn attack_next_arp(
        &mut self,
        at: Instant,
        next: &mut impl FnMut(Instant) -> Option<Vec<Vec<u8>>>,
    ) -> Option<[u8; 3]> {
        let mut sequence = arp_sequence(self.current_repeat_chord());
        if sequence.is_empty() {
            return None;
        }
        if self.arp_next_index >= sequence.len() {
            self.advance_progression(at, next);
            self.arp_next_index = 0;
            sequence = arp_sequence(self.current_repeat_chord());
        }
        let index = self.arp_next_index;
        let note = sequence[index];
        self.arp_next_index += 1;
        self.arp_sounding = Some(note);
        self.sounding.record(
            at,
            SoundingPosition {
                chord_index: self.repeat_chord_index,
                arp: Some(ArpStep { index, note }),
            },
        );
        Some(note_on(note.midi_note, self.velocity))
    }

    pub(super) fn current_repeat_chord(&self) -> &[PlaybackNote] {
        self.playback_progression()
            .get(self.repeat_chord_index)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(super) fn reset_progression_position(&mut self) {
        self.repeat_chord_index = 0;
        self.arp_next_index = 0;
    }
}

/// 和音を arp で鳴らす順: 昇順に並べ、続けて同じ音を 1 octave 上で繰り返す。
pub(crate) fn arp_sequence(chord: &[PlaybackNote]) -> Vec<PlaybackNote> {
    let mut base = chord.to_vec();
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

#[cfg(test)]
mod tests;
