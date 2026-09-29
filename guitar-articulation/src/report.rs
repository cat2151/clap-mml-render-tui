use std::fmt::Write as _;

use crate::ui::{event_text, note_name};
use crate::{
    articulate, convert, notes_from_events, strings_by_column, Articulated, RuleTable, PATCH,
};

/// 画面と同じ経路（`timed_performance` → [`convert`]）で作った演奏を、音ごとの表と
/// Articulated のイベント列の文字列にする。画面を開かずに変換の結果を読むためのもの。
pub fn report(mml: &str, rules: &RuleTable) -> Result<String, String> {
    let plain = cmrt_chord::timed_performance(mml)?.events;
    let notes = notes_from_events(&plain);
    let strings = strings_by_column(&notes);
    let articulated = articulate(&notes, rules);
    let converted = convert(&plain, rules);

    let mut out = String::new();
    let _ = writeln!(out, "mml: {mml:?}");
    let _ = writeln!(out, "rules: {}", rules.to_json());
    let _ = writeln!(out, "patch: {PATCH:?}");
    let _ = writeln!(out);
    let _ = writeln!(out, "# notes ({})", notes.len());
    let _ = writeln!(
        out,
        "{:>3} {:>3} {:>7} {:>7} {:<4} {:>3} {:>3} {:<10} accent",
        "col", "str", "on", "off", "note", "vel", "out", "stroke"
    );
    for (note, a) in notes.iter().zip(&articulated) {
        let line = format!(
            "{:>3} {:>3} {:>7.3} {:>7.3} {:<4} {:>3} {:>3} {:<10} {}",
            note.column,
            strings[note.column],
            note.on_seconds,
            note.off_seconds,
            note_name(note.pitch),
            note.velocity,
            a.velocity,
            a.articulation.name(),
            accent_mark(a),
        );
        let _ = writeln!(out, "{}", line.trim_end());
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "# converted events ({})", converted.len());
    for event in &converted {
        let _ = writeln!(out, "{}", event_text(event, true));
    }
    Ok(out)
}

/// 同じ MML を 2 つのルール表で変換し、音ごとの奏法と velocity を左右に並べる。違う行に `<>` を付ける。
pub fn compare(mml: &str, before: &RuleTable, after: &RuleTable) -> Result<String, String> {
    let plain = cmrt_chord::timed_performance(mml)?.events;
    let notes = notes_from_events(&plain);
    let strings = strings_by_column(&notes);
    let left = articulate(&notes, before);
    let right = articulate(&notes, after);

    let mut out = String::new();
    let _ = writeln!(out, "mml: {mml:?}");
    let _ = writeln!(out, "before: {}", before.to_json());
    let _ = writeln!(out, "after:  {}", after.to_json());
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{:>3} {:>3} {:<4} | {:<10} {:>3} {:<1} | {:<10} {:>3} {:<1}",
        "col", "str", "note", "before", "vel", "*", "after", "vel", "*"
    );
    for ((note, l), r) in notes.iter().zip(&left).zip(&right) {
        let line = format!(
            "{:>3} {:>3} {:<4} | {} | {} {}",
            note.column,
            strings[note.column],
            note_name(note.pitch),
            side(l),
            side(r),
            if l == r { "" } else { "<>" },
        );
        let _ = writeln!(out, "{}", line.trim_end());
    }
    let changed = left.iter().zip(&right).filter(|(l, r)| l != r).count();
    let _ = writeln!(out);
    let _ = writeln!(out, "changed: {changed} / {} notes", notes.len());
    Ok(out)
}

fn side(a: &Articulated) -> String {
    format!(
        "{:<10} {:>3} {:<1}",
        a.articulation.name(),
        a.velocity,
        accent_mark(a)
    )
}

fn accent_mark(a: &Articulated) -> &'static str {
    if a.accent {
        "*"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests;
