use crate::column_sound::apply_column_sounds;
use crate::{
    apply_glide_rules, apply_hammer_pull, apply_voicing_rules, strings_by_column, Articulation,
    Note, RowRule, RuleTable, TimedMidiEvent,
};

/// 強弱を付けるとき、アクセントでないピッキングの音の velocity を元の何 % にするか。
/// MML の既定 velocity は上限の 127 なので、アクセントを上げる代わりに他を下げて差を付ける。
/// METAL-GTX Lite の実測で、Sus の 95 は H-On / P-Off の 127 とほぼ同じ音量（どちらも Sus 127 の約 −4 dB）。
pub const UNACCENTED_PICK_VELOCITY_PERCENT: u16 = 75;

/// 1 音の奏法と強さ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Articulated {
    pub articulation: Articulation,
    /// アクセントの音（[`accents`]）か。強弱を付けるとき（[`articulate`]）だけ付く。
    pub accent: bool,
    pub velocity: u8,
    /// 鳴らす音高。`None` はその音を鳴らさない（ピックスクレイプや効果音の列の、最低音以外）。
    pub pitch: Option<u8>,
}

/// 音ごとの奏法と強さを、ルール表から決める。
///
/// [`RowRule::EconomyPicking`] が ON なら、ピッキングする音をイングヴェイ流の
/// エコノミーピッキングでダウン/アップへ振り分ける。
/// - 同じ弦（[`strings_by_column`]）の中はオルタネイト。
/// - 高い音の弦へ移るときはダウン。直前もダウンならスイープになる。
/// - 低い音の弦へ移るときは、移る前の弦の最後の音をプリングオフにし、次の弦をダウンで入る
///   （2 音の弦は D P → D、3 音の弦は D U P → D）。プリングの間にピックを次の弦の上へ戻せる。
///   最後の音がプリングにできない（弦に 1 音だけ・上行で終わる）ならオルタネイトのまま。
/// - 先頭と、和音の列の直後はダウンから始める。和音の列はダウンのまま。
///
/// [`RowRule::AutoHammerPull`] が ON なら、ピッキングする単音だけを数えてダウン/アップを交互にする
/// （振り出しはエコノミーピッキングと同じ）。
///
/// エコノミーピッキングの前に、列ごとのスライド・チョーキング（[`apply_glide_rules`]）を決める。
/// これらの音はピッキングとして数えない。
/// 最後に、列ごとの「どう鳴らすか」のルール（[`apply_voicing_rules`]）でピッキングする音の奏法を写し替え、
/// 列の最低音 1 つだけを鳴らすルール（ピックスクレイプ・クロマチックラン・スライドエフェクト・効果音）の列は
/// どの奏法でも上書きする。
///
/// エコノミーピッキングか汚し（[`RowRule::Humanize`]）が ON なら強弱を付ける。velocity はここで 1 回だけ
/// 決め、汚しは散らすだけ（割合を重ねると H-On / P-Off が聞こえないほど小さくなる）。
/// アクセントでないピッキングの音だけ [`UNACCENTED_PICK_VELOCITY_PERCENT`] へ下げ、H-On / P-Off・
/// ミュート・PH などは元のまま残す（サンプル自体が小さい）。効果音など音高を差し替えた音も元のまま
/// （スライドエフェクト Down は velocity で層を選ぶので、MML の値をそのまま使う）。
pub fn articulate(notes: &[Note], rules: &RuleTable) -> Vec<Articulated> {
    let mut articulations = apply_hammer_pull(notes, rules);
    apply_glide_rules(notes, rules, &mut articulations);
    if rules.is_row_on(RowRule::EconomyPicking) {
        apply_economy_picking(notes, &mut articulations);
    } else if rules.is_row_on(RowRule::AutoHammerPull) {
        apply_alternate_picking(notes, &mut articulations);
    }
    let dynamics = rules.is_row_on(RowRule::EconomyPicking) || rules.is_row_on(RowRule::Humanize);
    let accents = if dynamics {
        accents(notes)
    } else {
        vec![false; notes.len()]
    };
    apply_voicing_rules(notes, rules, &mut articulations);
    let pitches = apply_column_sounds(notes, rules, &mut articulations);
    notes
        .iter()
        .zip(articulations)
        .zip(accents)
        .zip(pitches)
        .map(|(((note, articulation), accent), pitch)| Articulated {
            articulation,
            accent,
            pitch,
            velocity: if dynamics
                && !accent
                && is_plain_pick(articulation)
                && pitch == Some(note.pitch)
            {
                unaccented_velocity(note.velocity)
            } else {
                note.velocity
            },
        })
        .collect()
}

/// ピッキングして、サンプルが Sus と同じ大きさで鳴る奏法。
fn is_plain_pick(articulation: Articulation) -> bool {
    match articulation {
        Articulation::SusDown
        | Articulation::SusUp
        | Articulation::SlideUp
        | Articulation::SlideDown
        | Articulation::BendHalf
        | Articulation::BendWhole
        | Articulation::BendWholeHalf
        | Articulation::SlideOut
        | Articulation::SlideIn
        | Articulation::TrillHalf
        | Articulation::TrillWhole
        | Articulation::TrillMinorThird
        | Articulation::TrillMajorThird
        | Articulation::UnisonBendAuto
        | Articulation::UnisonBendManual => true,
        Articulation::HammerOn
        | Articulation::PullOff
        | Articulation::MuteDown
        | Articulation::MuteUp
        | Articulation::PinchHarmonic
        | Articulation::PickScratch
        | Articulation::NaturalHarmonics
        | Articulation::BrushDown
        | Articulation::BrushUp
        | Articulation::MuteFretDown
        | Articulation::MuteFretUp
        | Articulation::PseudoLegato
        | Articulation::Portamento
        | Articulation::ChromaticRun
        | Articulation::SlideFxDown
        | Articulation::SlideFxUp
        | Articulation::SlideFxWow => false,
    }
}

fn unaccented_velocity(velocity: u8) -> u8 {
    let scaled = u16::from(velocity) * UNACCENTED_PICK_VELOCITY_PERCENT / 100;
    u8::try_from(scaled).unwrap_or(u8::MAX).max(1)
}

/// note on なら、その音の [`Articulated::velocity`]。
pub(crate) fn velocity_for(
    event: &TimedMidiEvent,
    notes: &[Note],
    articulated: &[Articulated],
) -> Option<u8> {
    let [status, pitch, velocity] = event.message;
    if status & 0xF0 != 0x90 || velocity == 0 {
        return None;
    }
    notes
        .iter()
        .zip(articulated)
        .find(|(note, _)| {
            note.channel == status & 0x0F
                && note.pitch == pitch
                && (note.on_seconds - event.seconds).abs() < 1e-9
        })
        .map(|(_, a)| a.velocity)
}

/// 列ごとの `notes` の添字の範囲。`notes` は列順を前提にする。
fn columns(notes: &[Note]) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    for chunk in notes.chunk_by(|a, b| a.column == b.column) {
        out.push(start..start + chunk.len());
        start += chunk.len();
    }
    out
}

fn apply_economy_picking(notes: &[Note], articulations: &mut [Articulation]) {
    let strings = strings_by_column(notes);
    let string_of = |i: usize| strings[notes[i].column];
    // ピッキングした音の (添字, ストローク)。和音の列を挟むと振り出しへ戻す。
    let mut picks: Vec<(usize, Articulation)> = Vec::new();
    // 直前の単音の添字。
    let mut previous: Option<usize> = None;
    for range in columns(notes) {
        if range.len() != 1 {
            picks.clear();
            previous = None;
            continue;
        }
        let i = range.start;
        if articulations[i] == Articulation::SusDown {
            let stroke = match (picks.last().copied(), previous) {
                (None, _) => Articulation::SusDown,
                (Some((last, stroke)), _) if string_of(last) == string_of(i) => opposite(stroke),
                (Some(_), Some(p)) if notes[i].pitch > notes[p].pitch => Articulation::SusDown,
                (Some((last, _)), Some(p)) if last != p => Articulation::SusDown,
                (Some(_), Some(p)) if can_pull_off(notes, &strings, p) => {
                    articulations[p] = Articulation::PullOff;
                    picks.pop();
                    Articulation::SusDown
                }
                (Some((_, stroke)), _) => opposite(stroke),
            };
            articulations[i] = stroke;
            picks.push((i, stroke));
        }
        previous = Some(i);
    }
}

fn apply_alternate_picking(notes: &[Note], articulations: &mut [Articulation]) {
    let mut stroke = Articulation::SusDown;
    for range in columns(notes) {
        if range.len() != 1 {
            stroke = Articulation::SusDown;
            continue;
        }
        let i = range.start;
        if articulations[i] == Articulation::SusDown {
            articulations[i] = stroke;
            stroke = opposite(stroke);
        }
    }
}

/// `p` を、同じ弦の 1 つ前の音からのプリングオフにできるか。
fn can_pull_off(notes: &[Note], strings: &[usize], p: usize) -> bool {
    p > 0
        && notes[p - 1].column + 1 == notes[p].column
        && strings[notes[p - 1].column] == strings[notes[p].column]
        && notes[p].pitch < notes[p - 1].pitch
}

fn opposite(stroke: Articulation) -> Articulation {
    if stroke == Articulation::SusDown {
        Articulation::SusUp
    } else {
        Articulation::SusDown
    }
}

/// アクセントの音。フレーズの頭（先頭の列の音）と、上行から下行へ折り返す頂点。
pub(crate) fn accents(notes: &[Note]) -> Vec<bool> {
    let mut out = apexes(notes);
    for (accent, note) in out.iter_mut().zip(notes) {
        *accent |= note.column == 0;
    }
    out
}

/// 前後の単音の列より高い単音。
fn apexes(notes: &[Note]) -> Vec<bool> {
    let mut out = vec![false; notes.len()];
    let columns = columns(notes);
    for window in columns.windows(3) {
        if window.iter().any(|range| range.len() != 1) {
            continue;
        }
        let [before, apex, after] = [0, 1, 2].map(|k| notes[window[k].start].pitch);
        if apex > before && apex > after {
            out[window[1].start] = true;
        }
    }
    out
}

#[cfg(test)]
mod tests;
