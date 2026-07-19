use crossterm::event::KeyEvent;
use midly::{MidiMessage, Smf, TrackEventKind};
use tui_textarea::TextArea;

pub(crate) struct KeyboardMmlInput<'a> {
    active: bool,
    textarea: TextArea<'a>,
    last_confirmed: String,
    error: Option<String>,
}

impl Default for KeyboardMmlInput<'_> {
    fn default() -> Self {
        Self {
            active: false,
            textarea: crate::text_input::new_single_line_textarea(""),
            last_confirmed: String::new(),
            error: None,
        }
    }
}

impl<'a> KeyboardMmlInput<'a> {
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn textarea(&self) -> &TextArea<'a> {
        &self.textarea
    }

    pub(crate) fn value(&self) -> String {
        crate::text_input::textarea_value(&self.textarea)
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(in crate::tui) fn open(&mut self) {
        self.textarea = crate::text_input::new_single_line_textarea(&self.last_confirmed);
        self.error = None;
        self.active = true;
    }

    pub(in crate::tui) fn cancel(&mut self) {
        self.active = false;
        self.error = None;
    }

    pub(in crate::tui) fn input(&mut self, key: KeyEvent) {
        self.error = None;
        self.textarea.input(key);
    }

    pub(in crate::tui) fn confirm(&mut self) -> Option<Vec<u8>> {
        let mml = self.value();
        match extract_note_numbers(&mml) {
            Ok(notes) => {
                self.last_confirmed = mml;
                self.error = None;
                self.active = false;
                Some(notes)
            }
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }
}

fn extract_note_numbers(mml: &str) -> Result<Vec<u8>, String> {
    if mml.trim().is_empty() {
        return Err("MMLを入力してください".to_string());
    }
    let preprocessed = chord2mml_core::convert(mml).unwrap_or_else(|_| mml.to_string());
    let bytes = mmlabc_to_smf::mml_to_smf_bytes(&preprocessed)
        .map_err(|error| format!("MML変換に失敗しました: {error}"))?;
    extract_note_numbers_from_smf(&bytes)
}

fn extract_note_numbers_from_smf(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let smf = Smf::parse(bytes).map_err(|error| format!("SMF解析に失敗しました: {error}"))?;
    let mut note_ons = Vec::new();
    for (track_index, track) in smf.tracks.iter().enumerate() {
        let mut absolute_time = 0_u64;
        for (event_index, event) in track.iter().enumerate() {
            absolute_time = absolute_time.saturating_add(u64::from(event.delta.as_int()));
            if let TrackEventKind::Midi {
                message: MidiMessage::NoteOn { key, vel },
                ..
            } = event.kind
            {
                if vel.as_int() > 0 {
                    note_ons.push((absolute_time, track_index, event_index, key.as_int()));
                }
            }
        }
    }
    note_ons.sort_unstable_by_key(|&(time, track, event, _)| (time, track, event));

    let mut seen = [false; 128];
    let mut notes = Vec::new();
    for (_, _, _, note) in note_ons {
        if !seen[usize::from(note)] {
            seen[usize::from(note)] = true;
            notes.push(note);
        }
    }
    if notes.is_empty() {
        return Err("MMLに発音ノートがありません".to_string());
    }
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use midly::{Format, Header, Timing, TrackEvent, TrackEventKind};

    #[test]
    fn mml_notes_are_extracted_in_order_and_deduplicated() {
        assert_eq!(extract_note_numbers("cec").unwrap(), vec![60, 64]);
    }

    #[test]
    fn chord_notation_is_preprocessed_before_mml_parsing() {
        assert_eq!(extract_note_numbers("C").unwrap(), vec![60, 64, 67]);
    }

    #[test]
    fn regular_mml_passes_through_when_chord_conversion_fails() {
        assert_eq!(extract_note_numbers("ceg").unwrap(), vec![60, 64, 67]);
    }

    #[test]
    fn empty_and_rest_only_mml_are_rejected() {
        assert_eq!(
            extract_note_numbers(" ").unwrap_err(),
            "MMLを入力してください"
        );
        assert_eq!(
            extract_note_numbers("r").unwrap_err(),
            "MMLに発音ノートがありません"
        );
    }

    #[test]
    fn smf_tracks_are_merged_by_absolute_time_and_zero_velocity_is_ignored() {
        let header = Header::new(Format::Parallel, Timing::Metrical(480.into()));
        let tracks = vec![
            vec![
                note_on(20, 67, 100),
                note_on(0, 69, 0),
                TrackEvent {
                    delta: 0.into(),
                    kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
                },
            ],
            vec![
                note_on(10, 60, 100),
                note_on(20, 67, 100),
                TrackEvent {
                    delta: 0.into(),
                    kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
                },
            ],
        ];
        let mut bytes = Vec::new();
        Smf { header, tracks }.write_std(&mut bytes).unwrap();

        assert_eq!(extract_note_numbers_from_smf(&bytes).unwrap(), vec![60, 67]);
    }

    fn note_on(delta: u32, key: u8, velocity: u8) -> TrackEvent<'static> {
        TrackEvent {
            delta: delta.into(),
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: MidiMessage::NoteOn {
                    key: key.into(),
                    vel: velocity.into(),
                },
            },
        }
    }
}
