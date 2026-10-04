use std::collections::BTreeSet;
use std::ops::RangeInclusive;

mod vibrato;

use crate::release::{RELEASE_SHAPE_CC, RELEASE_SHAPE_DEFAULT};
use crate::{
    slide_in_width, slide_semitones, Articulation, Note, RowRule, Rule, RuleTable, TimedMidiEvent,
    SLIDE_MAX_SEMITONES,
};

/// スライドの幅を選ぶ CC（METAL-GTX の `Slide_Range`）。
pub const SLIDE_WIDTH_CC: u8 = 26;

/// sfz の `set_cc26`。演奏の終わりにこの値へ戻す。
pub const SLIDE_WIDTH_CC_DEFAULT: u8 = 19;

/// スライドインの幅を選ぶ CC（METAL-GTX の `Slide_In_Range`）。区切りは CC26 と同じ 16 刻み。
pub const SLIDE_IN_WIDTH_CC: u8 = 27;

/// sfz の `set_cc27`。演奏の終わりにこの値へ戻す。
pub const SLIDE_IN_WIDTH_CC_DEFAULT: u8 = 82;

/// ビブラートの深さを決める CC（sfz の `pitchlfo_depthcc20=100`、127 で 100 cent）。
pub const VIBRATO_DEPTH_CC: u8 = 20;

/// [`Rule::Vibrato`] の最終深さの既定値。
pub const VIBRATO_DEPTH: u8 = 64;

/// sfz の `set_cc20`。ビブラートの音の終わりと演奏の終わりにこの値へ戻す。
const VIBRATO_DEPTH_CC_DEFAULT: u8 = 0;

/// Sus_Down を Sus_LT（velocity 111 以下）/ Sus_EX（112 以上）に、Mute_Down を Mute_EX にする CC
/// （sfz の `locc23=65`）。[`Rule::LongExtra`] の列で 127 を送る。
pub const LONG_EXTRA_CC: u8 = 23;

/// 5 度上（+7 半音）の層を Sus_Down / Sus_Up に重ねる CC（sfz の `Sus_P5`、`locc32=64`）。
/// [`Rule::PowerChord`] の列で 127 を送る。
pub const POWER_CHORD_CC: u8 = 32;

/// 5 度上の層が在る音高（sfz の `Sus_Down_P5` / `Sus_Up_P5` / `Sus_LT_P5` の region、F#1〜E6）。
pub const POWER_CHORD_PITCHES: RangeInclusive<u8> = 30..=88;

/// [`Rule::LongExtra`] と [`Rule::PowerChord`] の列で送る値。
const SWITCH_ON: u8 = 127;

/// sfz の `set_cc23` / `set_cc32` の既定（どちらも無指定で 0）。
const SWITCH_OFF: u8 = 0;

/// [`Rule::PositionRelease`] の列で送る CC24 の値。CC24 の 64〜79 の帯（手のポジション移動の離し音）の中。
pub const POSITION_RELEASE_VALUE: u8 = 72;

/// ポジション移動の離し音が在る音高（sfz の release5 の region）。
pub const POSITION_RELEASE_PITCHES: RangeInclusive<u8> = 30..=76;

/// [`Rule::AutoSlideOut`] の列で送る CC24 の値。CC24 の 80〜95 の帯（自動スライドアウト）の中。
/// 96 以上は Sus_Down の離し音がアップストロークになる（自動オルタネイト）。
pub const AUTO_SLIDE_OUT_VALUE: u8 = 88;

/// 自動スライドアウトの離し音が在る音高（sfz の release6 の region、G#1〜E6。Pull-Off だけ D#6 まで）。
pub const AUTO_SLIDE_OUT_PITCHES: RangeInclusive<u8> = 32..=88;

/// 列の頭で値を送り、列の最後の note off で既定へ戻す CC 1 つ。
pub(crate) struct ColumnCc {
    pub controller: u8,
    /// sfz の既定。
    pub default: u8,
    /// (ルール, 列で送る値)。同じ列で ON にできるのは 1 つだけ（[`Rule::selects_release_shape`]）。
    pub rules: &'static [(Rule, u8)],
}

pub(crate) const COLUMN_CCS: [ColumnCc; 3] = [
    ColumnCc {
        controller: LONG_EXTRA_CC,
        default: SWITCH_OFF,
        rules: &[(Rule::LongExtra, SWITCH_ON)],
    },
    ColumnCc {
        controller: POWER_CHORD_CC,
        default: SWITCH_OFF,
        rules: &[(Rule::PowerChord, SWITCH_ON)],
    },
    ColumnCc {
        controller: RELEASE_SHAPE_CC,
        default: RELEASE_SHAPE_DEFAULT,
        rules: &[
            (Rule::PositionRelease, POSITION_RELEASE_VALUE),
            (Rule::AutoSlideOut, AUTO_SLIDE_OUT_VALUE),
        ],
    },
];

/// CC を送るルールが、奏法 `articulation` で音高 `pitch` の音に効くか。
/// CC を送らないルールは `false`。ビブラートはどの音にも効く。
pub(crate) fn control_rule_affects(rule: Rule, articulation: Articulation, pitch: u8) -> bool {
    use Articulation as A;
    match rule {
        Rule::Vibrato => true,
        Rule::LongExtra => matches!(articulation, A::SusDown | A::MuteDown),
        Rule::PowerChord => {
            matches!(articulation, A::SusDown | A::SusUp) && POWER_CHORD_PITCHES.contains(&pitch)
        }
        Rule::PositionRelease => {
            matches!(
                articulation,
                A::SusDown
                    | A::SusUp
                    | A::SlideUp
                    | A::SlideDown
                    | A::HammerOn
                    | A::PullOff
                    | A::SlideIn
                    | A::PseudoLegato
            ) && POSITION_RELEASE_PITCHES.contains(&pitch)
        }
        // Mute_Down の離し音は、80 以上でスライドアウトではなくアップストロークになる。
        Rule::AutoSlideOut => match articulation {
            A::PullOff => AUTO_SLIDE_OUT_PITCHES.contains(&pitch) && pitch < 88,
            A::SusDown
            | A::SusUp
            | A::SlideUp
            | A::SlideDown
            | A::HammerOn
            | A::SlideIn
            | A::PseudoLegato => AUTO_SLIDE_OUT_PITCHES.contains(&pitch),
            _ => false,
        },
        _ => false,
    }
}

/// CC24 を選ぶルール（[`Rule::selects_release_shape`]）が ON で、効く音（[`control_rule_affects`]）が在る列。
pub(crate) fn release_shape_columns(
    notes: &[Note],
    articulations: &[Articulation],
    rules: &RuleTable,
) -> BTreeSet<usize> {
    notes
        .iter()
        .zip(articulations)
        .filter(|(note, articulation)| {
            [Rule::PositionRelease, Rule::AutoSlideOut]
                .into_iter()
                .any(|rule| {
                    rules.is_on(note.column, rule)
                        && control_rule_affects(rule, **articulation, note.pitch)
                })
        })
        .map(|(note, _)| note.column)
        .collect()
}

/// 奏法の列とルール表から、音ごとの CC のイベント列を作る（元の音符と KS は含まない）。
///
/// - `Slide_Up` / `Slide_Down` の音: note on と同時刻に、前の列からの幅の CC26。
/// - `Slide_In` の音: note on と同時刻に、[`slide_in_width`] の幅の CC27。
/// - [`Rule::Vibrato`] の列: 実際の最早 note on から、共通設定の待機・立ち上がり・最終深さの CC20。
///   列の最後の note off で 0 に戻し、同じ channel の次列が先に始まれば次列へ引き渡す。
///   CC は channel 全体に効くので、和音の列でも深さ変化は列で 1 本だけ作る。
/// - [`Rule::LongExtra`] / [`Rule::PowerChord`] / [`Rule::PositionRelease`] / [`Rule::AutoSlideOut`] の列:
///   列の最早 note on と最後の note off に CC23 = 127 / CC32 = 127 / CC24 = [`POSITION_RELEASE_VALUE`] /
///   CC24 = [`AUTO_SLIDE_OUT_VALUE`]。列に効く音
///   （[`control_rule_affects`]）が無ければ送らない。
///   汚し（リリース）が ON なら、CC24 は列ごとに汚し（リリース）が送り直すので、列の終わりでは戻さない。
///
/// - 行全体のパラメータ: 既定と違う値だけ、いちばん早い note on で送る（[`crate::PARAMS`]）。
/// - `Unison_Bend_Manual` の列: pitch bend を上げて戻す（[`crate::UNISON_BEND_RISE_SECONDS`]）。
///
/// CC は演奏を跨いで残るので、1 つでも送ったら最後の note off の時刻で sfz の既定値へ戻す。
/// 頭の既定値は含まない。演奏の列を作る側が、汚し（リリース）を積んだ後に [`add_column_cc_defaults`] で足す。
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
    let mut out = Vec::new();
    let slide = width_events(notes, articulations, widths, SLIDE_WIDTH_CC, |a, width| {
        matches!(a, Articulation::SlideUp | Articulation::SlideDown)
            .then_some(width)
            .flatten()
    });
    extend_with_reset(&mut out, slide, end, SLIDE_WIDTH_CC, SLIDE_WIDTH_CC_DEFAULT);
    let slide_in = width_events(
        notes,
        articulations,
        widths,
        SLIDE_IN_WIDTH_CC,
        |a, width| (a == Articulation::SlideIn).then(|| slide_in_width(width)),
    );
    extend_with_reset(
        &mut out,
        slide_in,
        end,
        SLIDE_IN_WIDTH_CC,
        SLIDE_IN_WIDTH_CC_DEFAULT,
    );
    out.extend(vibrato::events(notes, rules, end));
    for cc in &COLUMN_CCS {
        let reset_each_column =
            !(cc.controller == RELEASE_SHAPE_CC && rules.is_row_on(RowRule::HumanizeRelease));
        let events = column_cc_events(notes, articulations, rules, cc, reset_each_column);
        extend_with_reset(&mut out, events, end, cc.controller, cc.default);
    }
    out.extend(crate::params::param_events(notes, rules));
    out.extend(crate::unison_bend::pitch_bend_events(
        notes,
        articulations,
        end,
    ));
    out
}

/// 演奏のいちばん早い note on の時刻に CC20 と [`COLUMN_CCS`] の CC がまだ無ければ、sfz の既定値を足す。
///
/// 演奏が最後の note off より前に次の演奏で上書きされると、終わりの戻しが届かずに前の演奏の値が残る。
/// 頭で既定値を送れば、ルールが OFF の演奏も前の演奏の値に左右されない。
/// 同時刻に同じ CC が 2 つあるとどちらが効くかが並び順任せになるので、頭の時刻にその CC が
/// 既に在れば（ON の列の値や汚し（リリース）の値）足さない。頭の時刻のその CC はちょうど 1 つになる。
/// `out` は列 CC と汚し（リリース）を積み終えた演奏の列。足した既定値は `out` の先頭に入る。
pub(crate) fn add_column_cc_defaults(out: &mut Vec<TimedMidiEvent>, notes: &[Note]) {
    let Some(first) = notes
        .iter()
        .min_by(|a, b| a.on_seconds.total_cmp(&b.on_seconds))
    else {
        return;
    };
    let defaults: Vec<TimedMidiEvent> =
        std::iter::once((VIBRATO_DEPTH_CC, VIBRATO_DEPTH_CC_DEFAULT))
            .chain(COLUMN_CCS.iter().map(|cc| (cc.controller, cc.default)))
            .filter(|&(controller, _)| {
                !out.iter().any(|e| {
                    e.seconds == first.on_seconds
                        && e.message[0] & 0xF0 == 0xB0
                        && e.message[1] == controller
                })
            })
            .map(|(controller, default)| {
                control_change(first.on_seconds, first.channel, controller, default)
            })
            .collect();
    out.splice(0..0, defaults);
}

/// `events` が空でなければ、積んだ後に `end` で `controller` を `default` へ戻す。
fn extend_with_reset(
    out: &mut Vec<TimedMidiEvent>,
    events: Vec<TimedMidiEvent>,
    end: f64,
    controller: u8,
    default: u8,
) {
    if let Some(channel) = events.first().map(|event| event.message[0] & 0x0F) {
        out.extend(events);
        out.push(control_change(end, channel, controller, default));
    }
}

/// `width_of` が幅を返した音ごとに、note on と同時刻にその幅を選ぶ `controller` を送る。
fn width_events(
    notes: &[Note],
    articulations: &[Articulation],
    widths: &[Option<u8>],
    controller: u8,
    width_of: impl Fn(Articulation, Option<u8>) -> Option<u8>,
) -> Vec<TimedMidiEvent> {
    notes
        .iter()
        .zip(articulations)
        .zip(widths)
        .filter_map(|((note, articulation), width)| {
            let width = width_of(*articulation, *width)?;
            Some(control_change(
                note.on_seconds,
                note.channel,
                controller,
                slide_width_value(width),
            ))
        })
        .collect()
}

/// `cc` のルールが ON で効く音が在る列ごとに、列のいちばん早い note on でそのルールの値、
/// `reset_each_column` なら列の最後の note off で既定を送る。
fn column_cc_events(
    notes: &[Note],
    articulations: &[Articulation],
    rules: &RuleTable,
    cc: &ColumnCc,
    reset_each_column: bool,
) -> Vec<TimedMidiEvent> {
    let mut out = Vec::new();
    let mut start = 0;
    while let Some(first) = notes.get(start) {
        let len = notes[start..]
            .iter()
            .take_while(|note| note.column == first.column)
            .count();
        let column = &notes[start..start + len];
        let column_articulations = &articulations[start..start + len];
        start += len;
        let Some(&(_, value)) = cc.rules.iter().find(|&&(rule, _)| {
            rules.is_on(first.column, rule)
                && column
                    .iter()
                    .zip(column_articulations)
                    .any(|(note, articulation)| {
                        control_rule_affects(rule, *articulation, note.pitch)
                    })
        }) else {
            continue;
        };
        let on = column.iter().map(|n| n.on_seconds).fold(f64::MAX, f64::min);
        let off = column.iter().map(|n| n.off_seconds).fold(0.0, f64::max);
        out.push(control_change(on, first.channel, cc.controller, value));
        if reset_each_column {
            out.push(control_change(
                off,
                first.channel,
                cc.controller,
                cc.default,
            ));
        }
    }
    out
}

/// 幅 `semitones`（1〜[`SLIDE_MAX_SEMITONES`]）を選ぶ CC26 / CC27 の値。sfz の区切り（16 刻み）の中央。
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
