//! 演奏へ MIDI filter を重ねる、**唯一の場所**。
//!
//! 変換そのものは `cmrt-midi-filter`（依存なしの純粋な変換ライブラリ）が持つ。ここが決めるのは
//! 「どの LFO を」「どの絶対秒の範囲へ」掛けるかだけ。
//!
//! **絶対秒がそのまま LFO の位相**（原点は timeline の 0 秒）。周回 k のイベントを
//! `k * loop_seconds` ずらしてから、そのずらした秒のまま filter を掛けるので、
//! 継ぎ足しの境目で位相が飛ばない。ループ長と LFO の周期が割り切れなくてもよい。
//!
//! filter は repeat の有無に関係なく掛かる。1 回だけ鳴らす行も同じ [`one_shot`] を通る。

use cmrt_chord::TimedMidiEvent;
use cmrt_midi_filter::{
    insert_control_change, override_note_velocity, shift, Span, TriangleLfo, MODULATION_CC,
};

use crate::line_play::FilterSettings;

#[cfg(test)]
mod tests;

/// LFO の 1 周。ユーザーの決定「4 秒周期でずっと繰り返す」。
///
/// 1 周の長さ（`loop_seconds`）とは無関係。フレーズが 1 秒でも 4 秒かけて開閉する。
const FILTER_PERIOD_SECONDS: f64 = 4.0;

/// 振幅。CC も velocity も同じ波形を使う（velocity 側は 1 へ丸められる）。
const FILTER_MIN: u8 = 0;
const FILTER_MAX: u8 = 127;

fn filter_lfo() -> TriangleLfo {
    TriangleLfo::new(FILTER_PERIOD_SECONDS, FILTER_MIN, FILTER_MAX)
}

/// 周回 k のイベント列を作る。ずらしてから filter を掛けるところまで。
///
/// `offset_seconds` は `k * loop_seconds`。`span_seconds` は「この周が占める長さ」で、
/// CC を差し込む範囲になる（note の位置とは独立に、周の頭から終わりまで刻む）。
pub(super) fn lap(
    cycle: &[TimedMidiEvent],
    filters: FilterSettings,
    offset_seconds: f64,
    span_seconds: f64,
) -> Vec<TimedMidiEvent> {
    let mut lap = shift(cycle, offset_seconds);
    apply(
        &mut lap,
        filters,
        Span::new(offset_seconds, offset_seconds + span_seconds),
    );
    lap
}

/// 1 回だけ鳴らす行。頭は必ず 0 秒なので、LFO も min から始まる。
///
/// 範囲は行そのものの長さ。`loop_seconds` は「最後のイベントまで」なので、
/// 念のため実際のイベントの最終秒とも突き合わせる。**どちらも 0 の行**
/// （全イベントが 0 秒に居る行）では範囲が空になり、CC は 1 つも入らない。
/// 掛けるべき時間が無いので、それでよい。
pub(super) fn one_shot(
    events: &[TimedMidiEvent],
    filters: FilterSettings,
    loop_seconds: f64,
) -> Vec<TimedMidiEvent> {
    let last = events
        .iter()
        .map(|event| event.seconds)
        .fold(0.0_f64, f64::max);
    lap(events, filters, 0.0, loop_seconds.max(last))
}

/// 絶対秒のまま filter を掛ける。`span` がそのまま LFO の位相になる。
///
/// velocity を先に掛けるのは、差し込んだ CC を note on と誤って上書きしないため……ではなく
/// （[`override_note_velocity`] は note on 以外に触らない）、単に走査するイベントを増やさないため。
fn apply(events: &mut Vec<TimedMidiEvent>, filters: FilterSettings, span: Span) {
    if filters.velocity {
        override_note_velocity(events, &filter_lfo());
    }
    if filters.modulation {
        // 差し込んだ後に並べ直すのは insert_control_change 側の責務。
        // CC は同時刻の note on より必ず前に来る（鳴り始めに modulation 値が乗る）。
        insert_control_change(events, MODULATION_CC, &filter_lfo(), span);
    }
}
