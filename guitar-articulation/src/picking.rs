use crate::{
    apply_hammer_pull, strings_by_column, Articulation, Note, RowRule, RuleTable, TimedMidiEvent,
};

/// エコノミーピッキングで、アクセントでない音の velocity を元の何 % にするか。
/// MML の既定 velocity は上限の 127 なので、頂点を上げる代わりに他を下げて差を付ける。
pub const ECONOMY_UNACCENTED_VELOCITY_PERCENT: u16 = 75;

/// 1 音の奏法と強さ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Articulated {
    pub articulation: Articulation,
    /// 上行→下行の頂点の音か。エコノミーピッキングが ON のときだけ付く。
    pub accent: bool,
    pub velocity: u8,
}

/// 音ごとの奏法と強さを、ルール表から決める。
///
/// [`RowRule::EconomyPicking`] が ON なら、ピッキングする音をイングヴェイ流の
/// エコノミーピッキングでダウン/アップへ振り分け、頂点の音だけ元の velocity のまま残す。
/// - 同じ弦（[`strings_by_column`]）の中はオルタネイト。
/// - 高い音の弦へ移るときはダウン。直前もダウンならスイープになる。
/// - 低い音の弦へ移るときは、移る前の弦の最後の音をプリングオフにし、次の弦をダウンで入る
///   （2 音の弦は D P → D、3 音の弦は D U P → D）。プリングの間にピックを次の弦の上へ戻せる。
///   最後の音がプリングにできない（弦に 1 音だけ・上行で終わる）ならオルタネイトのまま。
/// - 先頭と、和音の列の直後はダウンから始める。和音の列はダウンのまま。
pub fn articulate(notes: &[Note], rules: &RuleTable) -> Vec<Articulated> {
    let mut articulations = apply_hammer_pull(notes, rules);
    let economy = rules.is_row_on(RowRule::EconomyPicking);
    let accents = if economy {
        apply_economy_picking(notes, &mut articulations);
        apexes(notes)
    } else {
        vec![false; notes.len()]
    };
    notes
        .iter()
        .zip(articulations)
        .zip(accents)
        .map(|((note, articulation), accent)| Articulated {
            articulation,
            accent,
            velocity: if !economy || accent {
                note.velocity
            } else {
                unaccented_velocity(note.velocity)
            },
        })
        .collect()
}

fn unaccented_velocity(velocity: u8) -> u8 {
    let scaled = u16::from(velocity) * ECONOMY_UNACCENTED_VELOCITY_PERCENT / 100;
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

/// 前後の単音の列より高い単音（上行から下行へ折り返す頂点）。
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
