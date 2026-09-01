//! コードのvoicingを、chord追従trackのlane音高へ解決する純粋関数。
//!
//! Grid SequencerとDaily DAWが同じ規則を使うため、画面固有stateから分離している。

/// bass laneを音高へ解決する。lane 0はbass、lane 1は1オクターブ上。
pub fn bass_octave_note(bass: Option<u8>, lane: usize) -> Option<u8> {
    let bass = bass?;
    match lane {
        0 => Some(bass),
        1 => bass.checked_add(12).filter(|note| *note <= 127),
        _ => None,
    }
}

/// signedな`rotation`ぶん構成音を累積してずらす。
pub fn rotated_chord_voice(
    notes: &[u8],
    lane: usize,
    rotation: i8,
    voice_limit: usize,
) -> Option<u8> {
    let notes = &notes[..notes.len().min(voice_limit)];
    let rendered_voice_count = if notes.len() == 3 {
        voice_limit
    } else {
        notes.len()
    };
    if lane >= rendered_voice_count || notes.is_empty() {
        return None;
    }
    let note_count = i16::try_from(notes.len()).ok()?;
    let rotation = i16::from(rotation);
    let mut previous = None;
    for voice in 0..=lane {
        let sequence = rotation + i16::try_from(voice).ok()?;
        let note_index = usize::try_from(sequence.rem_euclid(note_count)).ok()?;
        let mut note = i16::from(notes[note_index]) + 12 * sequence.div_euclid(note_count);
        while previous.is_some_and(|previous| note <= previous) {
            note += 12;
        }
        if !(0..=127).contains(&note) {
            return None;
        }
        previous = Some(note);
    }
    previous.map(|note| note as u8)
}

/// `base`に最も近いコード構成音のnote number。同距離なら低い方を返す。
pub fn snap_to_chord(base: u8, classes: &[bool; 12]) -> u8 {
    if !classes.iter().any(|on| *on) {
        return base;
    }
    for distance in 0..=6 {
        if let Some(down) = base.checked_sub(distance) {
            if classes[usize::from(down % 12)] {
                return down;
            }
        }
        if let Some(up) = base.checked_add(distance) {
            if up <= 127 && classes[usize::from(up % 12)] {
                return up;
            }
        }
    }
    base
}
