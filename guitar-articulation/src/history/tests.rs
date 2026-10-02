use serde_json::json;

use super::*;
use crate::{ColumnRuleAnchor, RowRule, Rule};

fn with_temp_history<T>(name: &str, body: impl FnOnce() -> T) -> T {
    let tmp = cmrt_history::test_support::unique_test_dir(&format!("guitar_articulation_{name}"));
    let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
    let result = body();
    std::fs::remove_dir_all(&tmp).ok();
    result
}

fn entry(mml: &str) -> GuitarArticulationHistoryEntry {
    GuitarArticulationHistoryEntry {
        mml: mml.to_string(),
        ..Default::default()
    }
}

fn mmls(history: &GuitarArticulationHistory) -> Vec<&str> {
    history.entries.iter().map(|e| e.mml.as_str()).collect()
}

#[test]
fn save_then_load_returns_same_entries() {
    with_temp_history("roundtrip", || {
        let mut rules = RuleTable::default();
        rules.toggle(2, Rule::HammerPull);
        rules.toggle_row(RowRule::EconomyPicking);
        let anchor = ColumnRuleAnchor::new("e", &rules);
        let mut history = GuitarArticulationHistory::default();
        history.push_front(entry("cde"));
        history.push_front(GuitarArticulationHistoryEntry {
            mml: "efg".to_string(),
            rules,
            effect_chain: vec![json!({"name": "amp", "params": {"gain": 0.5}})],
            anchor: Some(anchor),
        });

        save_history(&history).unwrap();

        assert_eq!(load_history(), history);
    });
}

#[test]
fn load_returns_empty_when_file_is_missing() {
    with_temp_history("missing", || {
        let path = cmrt_history::guitar_articulation_history_file_path().unwrap();
        assert!(!path.exists());
        assert!(load_history().entries.is_empty());
    });
}

#[test]
fn load_returns_empty_when_json_is_broken() {
    with_temp_history("broken", || {
        let path = cmrt_history::guitar_articulation_history_file_path().unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{\"entries\": [").unwrap();
        assert!(load_history().entries.is_empty());
    });
}

#[test]
fn load_fills_missing_fields_with_defaults() {
    with_temp_history("defaults", || {
        let path = cmrt_history::guitar_articulation_history_file_path().unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"entries":[{"mml":"cde"}]}"#).unwrap();
        assert_eq!(load_history().entries, vec![entry("cde")]);
    });
}

#[test]
fn saved_file_has_documented_shape() {
    with_temp_history("shape", || {
        let mut history = GuitarArticulationHistory::default();
        history.push_front(entry("cde"));
        save_history(&history).unwrap();
        let path = cmrt_history::guitar_articulation_history_file_path().unwrap();
        let value: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(
            value,
            json!({"entries":[{
                "mml":"cde",
                "rules":{"columns":{},"rows":[]},
                "effect_chain":[]
            }]})
        );
    });
}

#[test]
fn push_front_puts_newest_first() {
    let mut history = GuitarArticulationHistory::default();
    history.push_front(entry("a"));
    history.push_front(entry("b"));
    assert_eq!(mmls(&history), vec!["b", "a"]);
}

#[test]
fn push_front_moves_existing_entry_to_front_without_growing() {
    let mut history = GuitarArticulationHistory::default();
    history.push_front(entry("a"));
    history.push_front(entry("b"));
    history.push_front(entry("c"));
    history.push_front(entry("a"));
    assert_eq!(mmls(&history), vec!["a", "c", "b"]);
}

#[test]
fn push_front_treats_different_settings_as_different_entries() {
    let mut history = GuitarArticulationHistory::default();
    history.push_front(entry("a"));
    let mut rules = RuleTable::default();
    rules.toggle_row(RowRule::AutoHammerPull);
    history.push_front(GuitarArticulationHistoryEntry {
        rules,
        ..entry("a")
    });
    assert_eq!(history.entries.len(), 2);
}

#[test]
fn push_front_drops_oldest_past_the_limit() {
    let mut history = GuitarArticulationHistory::default();
    for i in 0..=HISTORY_MAX_LEN {
        history.push_front(entry(&format!("c{i}")));
    }
    assert_eq!(history.entries.len(), HISTORY_MAX_LEN);
    assert_eq!(history.entries[0].mml, format!("c{HISTORY_MAX_LEN}"));
    assert!(history.entries.iter().all(|e| e.mml != "c0"));
    assert_eq!(history.entries.last().unwrap().mml, "c1");
}

#[test]
fn file_lives_next_to_chord_chart_with_expected_name() {
    with_temp_history("path", || {
        let path = cmrt_history::guitar_articulation_history_file_path().unwrap();
        let chord_chart = cmrt_history::chord_chart_file_path().unwrap();
        assert_eq!(
            path.file_name().unwrap(),
            "guitar_articulation_history.json"
        );
        assert_eq!(path.parent(), chord_chart.parent());
    });
}
