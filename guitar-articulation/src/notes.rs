use crate::TimedMidiEvent;

/// 同じ列とみなす note on 時刻の差。同じ tick から tempo map で求めた秒は一致するが、念のため幅を持たせる。
const SAME_COLUMN_SECONDS: f64 = 1e-9;

/// note on と、それに対応する note off を組にした 1 音。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    pub on_seconds: f64,
    pub off_seconds: f64,
    pub channel: u8,
    pub pitch: u8,
    pub velocity: u8,
    /// 同時刻の note on のまとまりの通し番号（0 始まり）。
    pub column: usize,
}

pub(crate) fn is_note_on(message: &[u8; 3]) -> bool {
    message[0] & 0xF0 == 0x90 && message[2] != 0
}

pub(crate) fn is_note_off(message: &[u8; 3]) -> bool {
    message[0] & 0xF0 == 0x80 || (message[0] & 0xF0 == 0x90 && message[2] == 0)
}

/// イベント列を、note on 時刻順の音符へまとめる。
///
/// note off は、同じ channel・音高で まだ閉じていない最も古い note on に対応させる。
/// 閉じない note on は、列の最後のイベント時刻で閉じる。
pub fn notes_from_events(events: &[TimedMidiEvent]) -> Vec<Note> {
    let mut sorted = events.to_vec();
    cmrt_midi_filter::sort_for_playback(&mut sorted);
    let end_seconds = sorted.last().map_or(0.0, |e| e.seconds);

    let mut notes: Vec<Note> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    for event in &sorted {
        let m = &event.message;
        let channel = m[0] & 0x0F;
        if is_note_on(m) {
            open.push(notes.len());
            notes.push(Note {
                on_seconds: event.seconds,
                off_seconds: end_seconds,
                channel,
                pitch: m[1],
                velocity: m[2],
                column: 0,
            });
        } else if is_note_off(m) {
            if let Some(pos) = open
                .iter()
                .position(|&i| notes[i].channel == channel && notes[i].pitch == m[1])
            {
                let i = open.remove(pos);
                notes[i].off_seconds = event.seconds;
            }
        }
    }

    let mut column = 0;
    for i in 1..notes.len() {
        if notes[i].on_seconds - notes[i - 1].on_seconds > SAME_COLUMN_SECONDS {
            column += 1;
        }
        notes[i].column = column;
    }
    notes
}

#[cfg(test)]
mod tests;
