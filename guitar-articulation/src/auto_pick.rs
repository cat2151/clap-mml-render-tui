use crate::{picks_string, strings_by_column, Note};

/// この音数以上続く上行・下行だけを、[`RUN_PICK_EVERY`] 音ごとにピッキングする。
/// 短い上下（トリルなど）は折り返すたびに頂点と谷になるので、数えるとレガートが消える。
pub const RUN_MIN_NOTES: usize = 4;

/// 長い上行・下行の中で、何音ごとにピッキングするか。
pub const RUN_PICK_EVERY: usize = 3;

/// 列ごとの、自動ハンマリング（[`crate::RowRule::AutoHammerPull`]）でピッキングする列か。
///
/// 弦の最初の音（[`picks_string`]）に加えて、[`RUN_MIN_NOTES`] 音以上続く上行・下行は、
/// 始まりから [`RUN_PICK_EVERY`] 音ごとと、折り返す頂点・谷をピッキングする。
/// 折り返しの音は前後の上行・下行で共有し、そこから数え直す。
/// 和音の列と同音の繰り返しは上行・下行を切る。
/// `notes` は [`crate::notes_from_events`] の出力（列順）を前提にする。
pub fn auto_pick_columns(notes: &[Note]) -> Vec<bool> {
    let strings = strings_by_column(notes);
    let mut out = run_picks(&single_pitches(notes));
    for (column, pick) in out.iter_mut().enumerate() {
        *pick |= picks_string(&strings, column);
    }
    out
}

/// 列ごとの単音の高さ。和音の列は `None`。
fn single_pitches(notes: &[Note]) -> Vec<Option<u8>> {
    notes
        .chunk_by(|a, b| a.column == b.column)
        .map(|column| match column {
            [note] => Some(note.pitch),
            _ => None,
        })
        .collect()
}

fn run_picks(pitches: &[Option<u8>]) -> Vec<bool> {
    let mut out = vec![false; pitches.len()];
    // 列 i から i + 1 への向き。
    let steps: Vec<Option<std::cmp::Ordering>> = pitches
        .windows(2)
        .map(|pair| match pair {
            [Some(a), Some(b)] if a != b => Some(b.cmp(a)),
            _ => None,
        })
        .collect();
    let mut first = 0;
    for run in steps.chunk_by(|a, b| a == b) {
        let last = first + run.len();
        if run[0].is_some() && last - first + 1 >= RUN_MIN_NOTES {
            for column in (first..last).step_by(RUN_PICK_EVERY) {
                out[column] = true;
            }
            out[last] = true;
        }
        first = last;
    }
    out
}

#[cfg(test)]
mod tests;
