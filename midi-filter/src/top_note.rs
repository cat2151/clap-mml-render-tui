//! 重なった音のうち、いちばん高い音だけを残す（単音化）。

use crate::{is_note_off, is_note_on, sort_for_playback, TimedMidiEvent};

#[cfg(test)]
mod tests;

/// note on と、対応する note off（無ければ `None`）の組。
struct PairedNote {
    on: TimedMidiEvent,
    off: Option<TimedMidiEvent>,
    /// 鳴っている区間の終わり。note off が無い音は列の最後の時刻。
    off_seconds: f64,
}

/// 単音化した結果、その音をどう出すか。
enum Outcome {
    Keep,
    Truncate(f64),
    Drop,
}

/// 各音を、それより高い音が鳴り始めた時刻で切り詰める。
///
/// - 鳴っている区間 `[on, off)` の中で、より高い音が鳴っている最初の時刻を `t` とする
/// - `t` が無ければそのまま、`t > on` なら note off を `t` へ前倒し、`t == on` なら音ごと削除する
/// - 切り詰めた音は、上の音が止んでも鳴り直さない
///
/// 「より高い音が鳴っている」は単音化した**後**の区間で判定する。上の音自身が
/// さらに上の音で切り詰められていれば、その後ろは鳴っていない扱い。
/// 音高の比較は channel をまたぐ。
///
/// note on と note off の対応は、同じ channel・音高でまだ閉じていない最も古い note on に
/// 当てる。対応する note off が無い音は列の最後の時刻まで鳴っている扱いにする。
/// note 以外のイベントはそのまま残す。結果は [`sort_for_playback`] の順。
pub fn keep_top_notes(events: &[TimedMidiEvent]) -> Vec<TimedMidiEvent> {
    let mut sorted = events.to_vec();
    sort_for_playback(&mut sorted);
    let end_seconds = sorted.last().map_or(0.0, |event| event.seconds);

    let mut others = Vec::new();
    let mut notes: Vec<PairedNote> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    for event in sorted {
        let message = &event.message;
        if is_note_on(message) {
            open.push(notes.len());
            notes.push(PairedNote {
                on: event,
                off: None,
                off_seconds: end_seconds,
            });
        } else if is_note_off(message) {
            let paired = open
                .iter()
                .position(|&index| same_key(&notes[index].on.message, message));
            if let Some(position) = paired {
                let index = open.remove(position);
                notes[index].off = Some(event);
                notes[index].off_seconds = event.seconds;
            }
            // 対応する note on の無い note off は、鳴らす音が無いので捨てる。
        } else {
            others.push(event);
        }
    }

    let outcomes = top_note_outcomes(&notes);
    let mut result = others;
    for (note, outcome) in notes.iter().zip(outcomes) {
        match outcome {
            Outcome::Drop => {}
            Outcome::Keep => {
                result.push(note.on);
                result.extend(note.off);
            }
            Outcome::Truncate(seconds) => {
                result.push(note.on);
                let message = match note.off {
                    Some(off) => off.message,
                    None => [0x80 | (note.on.message[0] & 0x0F), note.on.message[1], 0],
                };
                result.push(TimedMidiEvent { seconds, message });
            }
        }
    }
    sort_for_playback(&mut result);
    result
}

/// 高い音から順に、単音化した後の区間を決めていく。
fn top_note_outcomes(notes: &[PairedNote]) -> Vec<Outcome> {
    let mut order: Vec<usize> = (0..notes.len()).collect();
    order.sort_by_key(|&index| std::cmp::Reverse(notes[index].on.message[1]));

    let mut outcomes: Vec<Option<Outcome>> = notes.iter().map(|_| None).collect();
    // 確定した区間 `(pitch, on, off)`。削除した音は入れない。
    let mut sounding: Vec<(u8, f64, f64)> = Vec::new();
    for index in order {
        let note = &notes[index];
        let pitch = note.on.message[1];
        let on = note.on.seconds;
        let off = note.off_seconds;
        let first_higher = sounding
            .iter()
            .filter(|&&(other_pitch, other_on, other_off)| {
                other_pitch > pitch && other_on < other_off && other_on < off && other_off > on
            })
            .map(|&(_, other_on, _)| other_on.max(on))
            .min_by(f64::total_cmp);
        let outcome = match first_higher {
            None => Outcome::Keep,
            Some(t) if t > on => Outcome::Truncate(t),
            Some(_) => Outcome::Drop,
        };
        match outcome {
            Outcome::Keep => sounding.push((pitch, on, off)),
            Outcome::Truncate(t) => sounding.push((pitch, on, t)),
            Outcome::Drop => {}
        }
        outcomes[index] = Some(outcome);
    }
    outcomes
        .into_iter()
        .map(|outcome| outcome.expect("every note gets an outcome"))
        .collect()
}

/// 同じ channel・音高か。
fn same_key(a: &[u8; 3], b: &[u8; 3]) -> bool {
    a[0] & 0x0F == b[0] & 0x0F && a[1] == b[1]
}
