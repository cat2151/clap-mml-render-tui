pub(super) fn midi_note_name(note: u8) -> String {
    const PITCH_CLASSES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let pitch_class = PITCH_CLASSES[usize::from(note % 12)];
    let octave = i16::from(note / 12) - 1;
    format!("{pitch_class}{octave}")
}
