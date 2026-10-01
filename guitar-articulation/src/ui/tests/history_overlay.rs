use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Modifier;

use super::*;
use crate::{GuitarArticulationHistoryEntry, RowRule, Rule, RuleTable};

/// `text` が載っている行の y と、その行頭の x。
fn find_row(buffer: &Buffer, text: &str) -> Option<(u16, u16)> {
    rows_in(buffer, buffer.area)
        .iter()
        .enumerate()
        .find_map(|(y, row)| {
            let squeezed: String = row.chars().filter(|ch| !ch.is_whitespace()).collect();
            squeezed.contains(text).then(|| {
                let x = row.find('#').unwrap();
                (row[..x].chars().count() as u16, y as u16)
            })
        })
}

#[test]
fn the_overlay_lists_the_mml_of_each_entry_and_reverses_the_selected_row() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    screen.apply_history_entry(&GuitarArticulationHistoryEntry {
        mml: "o4 c d".to_string(),
        ..Default::default()
    });
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT));
    screen.handle_key_event(key(KeyCode::Char('j')));

    let buffer = render(&screen);

    let all = squeezed(&rows_in(&buffer, buffer.area));
    assert!(all.contains("GuitarArticulationHistory"), "{all}");
    let newest = find_row(&buffer, "#01o4cd").expect("#01 の行");
    let selected = find_row(&buffer, "#02o3l8egaH/P:1").expect("#02 の行");
    assert!(find_row(&buffer, "#03o3l8ega").is_some(), "{all}");
    assert!(!buffer
        .cell(newest)
        .unwrap()
        .modifier
        .contains(Modifier::REVERSED));
    assert_ne!(
        buffer.cell(selected).unwrap().style(),
        buffer.cell(newest).unwrap().style(),
        "選択行は他の行と違う style"
    );
}

#[test]
fn a_row_names_the_row_rules_the_column_rule_counts_and_the_chain_length() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    rules.toggle(2, Rule::HammerPull);
    rules.toggle(2, Rule::Vibrato);
    rules.toggle_row(RowRule::EconomyPicking);
    rules.toggle_row(RowRule::Humanize);
    rules.toggle_row(RowRule::HumanizeRelease);
    rules.step_param(22, -51);
    let entry = GuitarArticulationHistoryEntry {
        mml: "o3 l8 e g a".to_string(),
        rules,
        effect_chain: vec![serde_json::json!({}), serde_json::json!({})],
        anchor: None,
    };

    assert_eq!(
        history::row_text(0, &entry),
        "#01  o3 l8 e g a  eco 汚し 汚しrel H/P:2 vib:1 cc22=0  fx:2"
    );
    assert_eq!(
        history::row_text(9, &GuitarArticulationHistoryEntry::default()),
        "#10  "
    );
}
