use crate::{
    slide_semitones, Articulation, Note, Rule, RuleTable, TimedMidiEvent, SLIDE_MAX_SEMITONES,
};

/// スライドの幅を選ぶ CC（METAL-GTX の `Slide_Range`）。
pub const SLIDE_WIDTH_CC: u8 = 26;

/// sfz の `set_cc26`。演奏の終わりにこの値へ戻す。
pub const SLIDE_WIDTH_CC_DEFAULT: u8 = 19;

/// ビブラートの深さを決める CC（sfz の `pitchlfo_depthcc20=100`、127 で 100 cent）。
pub const VIBRATO_DEPTH_CC: u8 = 20;

/// [`Rule::Vibrato`] の列で送る CC20 の値。
pub const VIBRATO_DEPTH: u8 = 64;

/// sfz の `set_cc20`。ビブラートの音の終わりと演奏の終わりにこの値へ戻す。
const VIBRATO_DEPTH_CC_DEFAULT: u8 = 0;

/// 奏法の列とルール表から、音ごとの CC のイベント列を作る（元の音符と KS は含まない）。
///
/// - `Slide_Up` / `Slide_Down` の音: note on と同時刻に、前の列からの幅の CC26。
/// - [`Rule::Vibrato`] の列: 列のいちばん早い note on と同時刻に CC20 = [`VIBRATO_DEPTH`]、列の最後の note off と同時刻に 0。
///   CC は channel 全体に効くので、和音の列でも列で 1 回だけ送る。
///
/// CC は演奏を跨いで残るので、1 つでも送ったら最後の note off の時刻で sfz の既定値へ戻す。
/// 同時刻の CC は積んだ順に並ぶので、列の順に積めば、前の列の戻しが次の列の深さより前に来る。
pub fn control_events(
    notes: &[Note],
    articulations: &[Articulation],
    rules: &RuleTable,
) -> Vec<TimedMidiEvent> {
    let widths: Vec<Option<u8>> = (0..notes.len())
        .map(|i| slide_semitones(notes, i))
        .collect();
    control_events_with_widths(notes, articulations, &widths, rules)
}

/// [`control_events`] の、スライドの幅を外から渡す版。`widths[i]` は `notes[i]` の前の列からの音程
/// （[`slide_semitones`]）。前の列を含まない切り出し（1 列だけの `notes`）でも幅を保てる。
pub(crate) fn control_events_with_widths(
    notes: &[Note],
    articulations: &[Articulation],
    widths: &[Option<u8>],
    rules: &RuleTable,
) -> Vec<TimedMidiEvent> {
    let end = notes.iter().map(|n| n.off_seconds).fold(0.0, f64::max);
    let mut out = slide_width_events(notes, articulations, widths);
    if let Some(channel) = out.first().map(|event| event.message[0] & 0x0F) {
        out.push(control_change(
            end,
            channel,
            SLIDE_WIDTH_CC,
            SLIDE_WIDTH_CC_DEFAULT,
        ));
    }
    let vibrato = vibrato_events(notes, rules);
    if let Some(channel) = vibrato.first().map(|event| event.message[0] & 0x0F) {
        out.extend(vibrato);
        out.push(control_change(
            end,
            channel,
            VIBRATO_DEPTH_CC,
            VIBRATO_DEPTH_CC_DEFAULT,
        ));
    }
    out
}

fn slide_width_events(
    notes: &[Note],
    articulations: &[Articulation],
    widths: &[Option<u8>],
) -> Vec<TimedMidiEvent> {
    let mut out = Vec::new();
    for ((note, articulation), width) in notes.iter().zip(articulations).zip(widths) {
        if !matches!(
            articulation,
            Articulation::SlideUp | Articulation::SlideDown
        ) {
            continue;
        }
        let Some(width) = *width else {
            continue;
        };
        out.push(control_change(
            note.on_seconds,
            note.channel,
            SLIDE_WIDTH_CC,
            slide_width_value(width),
        ));
    }
    out
}

fn vibrato_events(notes: &[Note], rules: &RuleTable) -> Vec<TimedMidiEvent> {
    let mut out = Vec::new();
    let mut rest = notes;
    while let Some(first) = rest.first() {
        let len = rest
            .iter()
            .take_while(|note| note.column == first.column)
            .count();
        let (column, tail) = rest.split_at(len);
        rest = tail;
        if !rules.is_on(first.column, Rule::Vibrato) {
            continue;
        }
        let on = column.iter().map(|n| n.on_seconds).fold(f64::MAX, f64::min);
        let off = column.iter().map(|n| n.off_seconds).fold(0.0, f64::max);
        out.push(control_change(
            on,
            first.channel,
            VIBRATO_DEPTH_CC,
            VIBRATO_DEPTH,
        ));
        out.push(control_change(
            off,
            first.channel,
            VIBRATO_DEPTH_CC,
            VIBRATO_DEPTH_CC_DEFAULT,
        ));
    }
    out
}

/// 幅 `semitones`（1〜[`SLIDE_MAX_SEMITONES`]）を選ぶ CC26 の値。sfz の区切り（16 刻み）の中央。
fn slide_width_value(semitones: u8) -> u8 {
    (semitones.clamp(1, SLIDE_MAX_SEMITONES) - 1) * 16 + 8
}

fn control_change(seconds: f64, channel: u8, controller: u8, value: u8) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [0xB0 | channel, controller, value],
    }
}

#[cfg(test)]
mod tests;
