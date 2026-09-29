use crate::{strings_by_column, Articulation, Note, RuleTable};

/// H/P が効く列（[`RuleTable::hammer_pull_applies`]）の音を、前の列からの上下でハンマリング/プリングオフにする。
///
/// 対象は単音の列から単音の列へ移る所だけ。先頭の列・和音が絡む列・同音は既定の奏法のまま。
/// `notes` は [`crate::notes_from_events`] の出力（列順）を前提にする。
pub fn apply_hammer_pull(notes: &[Note], rules: &RuleTable) -> Vec<Articulation> {
    let mut out = vec![Articulation::SusDown; notes.len()];
    let strings = strings_by_column(notes);
    for (i, note) in notes.iter().enumerate() {
        if note.column == 0 || !rules.hammer_pull_applies(note.column, &strings) {
            continue;
        }
        let Some(current) = single_in_column(notes, note.column) else {
            continue;
        };
        let Some(previous) = single_in_column(notes, note.column - 1) else {
            continue;
        };
        out[i] = match current.pitch.cmp(&previous.pitch) {
            std::cmp::Ordering::Greater => Articulation::HammerOn,
            std::cmp::Ordering::Less => Articulation::PullOff,
            std::cmp::Ordering::Equal => Articulation::SusDown,
        };
    }
    out
}

fn single_in_column(notes: &[Note], column: usize) -> Option<&Note> {
    let mut in_column = notes.iter().filter(|n| n.column == column);
    let first = in_column.next()?;
    in_column.next().is_none().then_some(first)
}

#[cfg(test)]
mod tests;
