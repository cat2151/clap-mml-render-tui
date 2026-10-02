use super::*;
use crate::RowRule;
use cmrt_arpeggiator::ArpPattern;

fn economy() -> RuleTable {
    let mut rules = RuleTable::default();
    rules.toggle_row(RowRule::EconomyPicking);
    rules
}

#[test]
fn the_report_lists_each_note_with_its_stroke_and_the_converted_events() {
    let text = report("l16cdefgab<c", None, &economy()).unwrap();

    assert!(text.starts_with(
        "mml: \"l16cdefgab<c\"\nrules: {\"columns\":{},\"rows\":[\"economy_picking\"]}\n"
    ));
    assert!(text.contains("# notes (8)"));
    let strokes: Vec<&str> = text
        .lines()
        .skip_while(|line| !line.starts_with("col"))
        .skip(1)
        .take(8)
        .map(|line| line.split_whitespace().nth(7).unwrap())
        .collect();
    // 1 弦 3 音。高い弦へ移る頭はダウン（直前もダウンなのでスイープ）。
    assert_eq!(
        strokes,
        [
            "Sus_Down", "Sus_Up", "Sus_Down", "Sus_Down", "Sus_Up", "Sus_Down", "Sus_Down",
            "Sus_Up"
        ]
    );
    assert!(text.contains("KS Sus_Up"));
}

#[test]
fn the_report_rejects_an_unreadable_mml() {
    assert!(report("", None, &RuleTable::default()).is_err());
}

#[test]
fn compare_marks_only_the_notes_whose_stroke_or_velocity_changed() {
    let text = compare("l16cdefgab<c", None, &RuleTable::default(), &economy()).unwrap();

    assert!(
        text.contains("before: {\"columns\":{},\"rows\":[]}"),
        "{text}"
    );
    assert!(text.contains("after:  {\"columns\":{},\"rows\":[\"economy_picking\"]}"));
    // 全音の velocity が 127 → 95 に下がる。頂点が無いので全行が違う。
    assert!(text.contains("changed: 8 / 8 notes"), "{text}");
    let same = compare("l16cdefgab<c", None, &economy(), &economy()).unwrap();
    assert!(same.contains("changed: 0 / 8 notes"));
    assert!(!same.contains("<>"));
}

#[test]
fn the_report_converts_the_arpeggiated_performance_and_names_the_arp() {
    let arp = ArpSettings {
        pattern: ArpPattern::UpDown,
        ..ArpSettings::default()
    };

    let text = report("l16cdef", Some(&arp), &RuleTable::default()).unwrap();

    assert!(text.contains("# notes (6)"), "{text}");
    assert!(text.contains("arp: {\"pattern\":\"UpDown\""), "{text}");
    let off = report("l16cdef", None, &RuleTable::default()).unwrap();
    assert!(off.contains("# notes (4)"));
    assert!(!off.contains("arp:"));
    let compared = compare("l16cdef", Some(&arp), &RuleTable::default(), &economy()).unwrap();
    assert!(compared.contains("/ 6 notes"), "{compared}");
}
