use crate::{Articulation, Note, TimedMidiEvent};

pub const KEYSWITCH_VELOCITY: u8 = 127;

/// 末尾で既定の奏法へ戻す KS の長さ。押している音が無いので長さは効かないが、
/// note on と off を同時刻に置くと off が先に並んで KS が押しっぱなしになる。
pub const KEYSWITCH_RESET_SECONDS: f64 = 0.05;

/// 奏法の列から KS のイベント列を作る（元の音符は含まない）。
///
/// KS は `sw_last` のラッチ式で演奏を跨いで残るので、先頭で必ず既定の奏法を押し、
/// 最後が既定以外なら末尾で既定へ戻す。途中は奏法が変わる音の前にだけ置く。
/// KS はその音の列でいちばん早い note on の時刻に置き（和音の音がずれていても、列のどの音より前に効く）、
/// その音の note off で離す。
pub fn keyswitch_events(notes: &[Note], articulations: &[Articulation]) -> Vec<TimedMidiEvent> {
    let mut out = Vec::new();
    let mut latched: Option<Articulation> = None;
    for (note, &articulation) in notes.iter().zip(articulations) {
        let on_seconds = notes
            .iter()
            .filter(|other| other.column == note.column)
            .map(|other| other.on_seconds)
            .fold(note.on_seconds, f64::min);
        if latched.is_none() {
            // 先頭の音の奏法に関わらず、まず既定へ揃えてから変える。
            push_keyswitch(
                &mut out,
                note.channel,
                Articulation::SusDown,
                on_seconds,
                note.off_seconds,
            );
            latched = Some(Articulation::SusDown);
        }
        if latched != Some(articulation) {
            push_keyswitch(
                &mut out,
                note.channel,
                articulation,
                on_seconds,
                note.off_seconds,
            );
        }
        latched = Some(articulation);
    }

    if let (Some(last), Some(latched)) = (notes.last(), latched) {
        if latched != Articulation::SusDown {
            let end = notes
                .iter()
                .map(|n| n.off_seconds)
                .fold(last.off_seconds, f64::max);
            push_keyswitch(
                &mut out,
                last.channel,
                Articulation::SusDown,
                end,
                end + KEYSWITCH_RESET_SECONDS,
            );
        }
    }
    out
}

fn push_keyswitch(
    out: &mut Vec<TimedMidiEvent>,
    channel: u8,
    articulation: Articulation,
    on_seconds: f64,
    off_seconds: f64,
) {
    let key = articulation.keyswitch();
    out.push(TimedMidiEvent {
        seconds: on_seconds,
        message: [0x90 | channel, key, KEYSWITCH_VELOCITY],
    });
    out.push(TimedMidiEvent {
        seconds: off_seconds,
        message: [0x80 | channel, key, 0],
    });
}

#[cfg(test)]
mod tests;
