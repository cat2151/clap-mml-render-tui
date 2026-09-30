use crate::control::control_events_with_widths;
use crate::{
    keyswitch_events, slide_semitones, Articulated, Articulation, Note, RuleTable, Take,
    TimedMidiEvent,
};

/// `column` 列の音（和音なら列の全部）だけを、0 秒から鳴らすイベント列。
///
/// 奏法と velocity はフレーズ全体で決めた `articulated`（`notes` と同じ並び）を使う。
/// KS はラッチ式なので、全体の列を時刻で切り出さず、この列の音だけから
/// [`keyswitch_events`] で作り直す（先頭で既定を押し、既定以外なら末尾で戻す）。
/// CC も同じく、この列の音と `rules` から作り直す。スライドの幅だけは前の列の音から決める。
/// [`Take::Plain`] は KS も CC も含まず、velocity と音高も MML のまま。列が無ければ空。
pub fn column_events(
    notes: &[Note],
    articulated: &[Articulated],
    rules: &RuleTable,
    column: usize,
    take: Take,
) -> Vec<TimedMidiEvent> {
    let picked: Vec<(Note, &Articulated, Option<u8>)> = notes
        .iter()
        .zip(articulated)
        .enumerate()
        .filter(|(_, (note, _))| note.column == column)
        .map(|(i, (note, a))| (*note, a, slide_semitones(notes, i)))
        .collect();
    let Some(start) = picked.first().map(|(note, _, _)| note.on_seconds) else {
        return Vec::new();
    };
    let shifted: Vec<Note> = picked
        .iter()
        .map(|(note, _, _)| Note {
            on_seconds: note.on_seconds - start,
            off_seconds: note.off_seconds - start,
            ..*note
        })
        .collect();

    let mut out = match take {
        Take::Converted => {
            let articulations: Vec<Articulation> =
                picked.iter().map(|(_, a, _)| a.articulation).collect();
            let widths: Vec<Option<u8>> = picked.iter().map(|(_, _, width)| *width).collect();
            let mut out = keyswitch_events(&shifted, &articulations);
            out.extend(control_events_with_widths(
                &shifted,
                &articulations,
                &widths,
                rules,
            ));
            out
        }
        Take::Plain => Vec::new(),
    };
    for (note, (_, a, _)) in shifted.iter().zip(&picked) {
        let (velocity, pitch) = match take {
            Take::Converted => (a.velocity, a.pitch),
            Take::Plain => (note.velocity, Some(note.pitch)),
        };
        let Some(pitch) = pitch else {
            continue;
        };
        out.push(TimedMidiEvent {
            seconds: note.on_seconds,
            message: [0x90 | note.channel, pitch, velocity],
        });
        out.push(TimedMidiEvent {
            seconds: note.off_seconds,
            message: [0x80 | note.channel, pitch, 0],
        });
    }
    cmrt_midi_filter::sort_for_playback(&mut out);
    out
}

#[cfg(test)]
mod tests;
