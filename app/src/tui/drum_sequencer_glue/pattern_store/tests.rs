use cmrt_drum_sequencer::{DrumHit, DEFAULT_VELOCITY};

use super::*;

#[test]
fn edited_patterns_are_written_per_kit_and_broken_files_are_reported_but_skipped() {
    let _dirs = crate::history::test_support::temp_local_dirs("drum_pattern_store");
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("Kits/909.sfz".into(), Some(vec![36]), Vec::new(), vec![36]);
    // 編集が無ければ書かない。
    save_edited_pattern(&mut screen).unwrap();
    assert!(load_drum_pattern_files("Kits/909.sfz").unwrap().is_empty());

    let pattern = DrumPattern::from_hits([DrumHit {
        note: 36,
        step: 0,
        steps: 1,
        velocity: DEFAULT_VELOCITY,
    }]);
    screen.restore(Some("Kits/909.sfz".into()), [], 4, Some(36), 0);
    screen.set_kit("Kits/909.sfz".into(), Some(vec![36]), Vec::new(), vec![36]);
    screen.handle_key_event(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Char(' '),
    ));
    save_edited_pattern(&mut screen).unwrap();
    assert_eq!(
        load_kit_patterns("Kits/909.sfz"),
        (vec![(4, pattern)], None)
    );
    assert!(load_kit_patterns("Kits/other.sfz").0.is_empty());

    save_drum_pattern_file("Kits/909.sfz", 2, Some(b"broken")).unwrap();
    let (patterns, error) = load_kit_patterns("Kits/909.sfz");
    assert_eq!(patterns.len(), 1);
    assert!(error.unwrap().contains("pattern 2"));

    // 消して空になった pattern はファイルごと消える。
    screen.handle_key_event(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Char(' '),
    ));
    save_edited_pattern(&mut screen).unwrap();
    assert_eq!(
        load_drum_pattern_files("Kits/909.sfz")
            .unwrap()
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>(),
        [2]
    );
}
