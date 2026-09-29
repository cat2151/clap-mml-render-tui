use crate::Note;

/// 1 本の弦で押さえる音の幅（半音）。1 フレット 1 指の 4 フレットにストレッチ 1 つ。
/// 4 にするとメジャースケールが 1 弦 3 音、ペンタトニックが 1 弦 2 音（ボックス）になる。
/// 5 だとどちらも 1 本に詰めすぎる（ペンタが 1 弦 3 音になり、実際の運指と合わない）。
pub const REACH_SEMITONES: u8 = 4;

/// 列ごとの、何本目の弦で弾くか（先頭が 0。弦を移るたびに 1 増える通し番号で、実際の弦の番号ではない）。
///
/// 単音は、今の弦で押さえた音と合わせた幅が [`REACH_SEMITONES`] を超えたら次の弦へ移る。
/// 和音の列はそれだけで 1 本とし、次の単音から新しい弦を始める。
/// `notes` は [`crate::notes_from_events`] の出力（列順）を前提にする。
pub fn strings_by_column(notes: &[Note]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut string = 0;
    // 今の弦で押さえた単音の (最低音, 最高音)。和音の直後は `None`。
    let mut span: Option<(u8, u8)> = None;
    for column in notes.chunk_by(|a, b| a.column == b.column) {
        let single = match column {
            [note] => Some(note.pitch),
            _ => None,
        };
        span = match (span, single) {
            (Some((low, high)), Some(pitch))
                if high.max(pitch) - low.min(pitch) <= REACH_SEMITONES =>
            {
                Some((low.min(pitch), high.max(pitch)))
            }
            _ => {
                if !out.is_empty() {
                    string += 1;
                }
                single.map(|pitch| (pitch, pitch))
            }
        };
        out.push(string);
    }
    out
}

/// その列が弦の最初の音か。`strings` は [`strings_by_column`] の出力。
pub fn picks_string(strings: &[usize], column: usize) -> bool {
    column == 0 || strings.get(column - 1) != strings.get(column)
}

#[cfg(test)]
mod tests;
