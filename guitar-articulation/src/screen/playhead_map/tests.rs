use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{GuitarArticulationScreen, RowRule, Take, TimedMidiEvent};

const TAKES: [Take; 2] = [Take::Plain, Take::Converted];

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn press(screen: &mut GuitarArticulationScreen, ch: char) {
    screen.handle_key_event(key(KeyCode::Char(ch)));
}

fn screen_with_mml(mml: &str) -> GuitarArticulationScreen {
    let mut screen = GuitarArticulationScreen::default();
    press(&mut screen, 'i');
    for ch in mml.chars() {
        press(&mut screen, ch);
    }
    screen.handle_key_event(key(KeyCode::Enter));
    screen
}

/// 0.2 秒おきに 2 音の和音と単音を交互に鳴らす `count` 列の SMF 素材。
fn smf_screen_with_chords(count: usize) -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("o3 l8 e g a");
    let mut events = Vec::new();
    for i in 0..count {
        let seconds = i as f64 * 0.2;
        let pitches: &[u8] = if i % 2 == 0 { &[52, 59] } else { &[55] };
        for &pitch in pitches {
            events.push(TimedMidiEvent {
                seconds,
                message: [0x90, pitch, 100],
            });
            events.push(TimedMidiEvent {
                seconds: seconds + 0.1,
                message: [0x80, pitch, 0],
            });
        }
    }
    events.sort_by(|a, b| a.seconds.total_cmp(&b.seconds));
    screen.load_smf(PathBuf::from("chords.mid"), Ok(events));
    screen
}

fn is_note_on(event: &TimedMidiEvent) -> bool {
    event.message[0] & 0xF0 == 0x90 && event.message[2] != 0
}

/// 全列について、表から引いた行が `column_on_seconds` の秒に鳴るいちばん前の note on であること。
fn assert_rows_match(screen: &GuitarArticulationScreen) {
    for take in TAKES {
        let events = screen.events(take);
        for column in 0..screen.column_count() {
            let on = screen.column_on_seconds(column, take).unwrap();
            let expected = events
                .iter()
                .position(|event| is_note_on(event) && (event.seconds - on).abs() < 1e-9);
            let row = screen.playhead_map.row(take, column);
            assert!(row.is_some(), "{take:?} 列 {column}");
            assert_eq!(row, expected, "{take:?} 列 {column}");
            let event = &events[row.unwrap()];
            assert!(is_note_on(event));
            assert!((event.seconds - on).abs() < 1e-9, "{take:?} 列 {column}");
        }
    }
}

/// 走査で求めた、`seconds` までに鳴り始めた列のうち最も後ろの列。
fn scanned_column(screen: &GuitarArticulationScreen, take: Take, seconds: f64) -> Option<usize> {
    (0..screen.column_count())
        .filter(|&column| {
            screen
                .column_on_seconds(column, take)
                .is_some_and(|on| on <= seconds)
        })
        .max()
}

#[test]
fn rows_point_at_each_columns_note_on_with_humanize_off_and_on() {
    for mut screen in [
        screen_with_mml("o3 l16 e f+ g a b a g f+"),
        smf_screen_with_chords(12),
    ] {
        assert!(!screen.rules().is_row_on(RowRule::Humanize));
        assert_rows_match(&screen);

        press(&mut screen, 'd');
        assert!(screen.rules().is_row_on(RowRule::Humanize));
        assert_ne!(
            screen.column_on_seconds(1, Take::Converted),
            screen.column_on_seconds(1, Take::Plain),
            "汚しで Articulated の秒がずれている"
        );
        assert_rows_match(&screen);
    }
}

#[test]
fn the_column_is_found_at_its_start_just_after_it_and_not_before_the_first_note() {
    let screen = smf_screen_with_chords(12);
    for take in TAKES {
        let first = screen.column_on_seconds(0, take).unwrap();
        assert_eq!(screen.playhead_map.column(take, first - 0.001), None);
        for column in 0..screen.column_count() {
            let on = screen.column_on_seconds(column, take).unwrap();
            assert_eq!(screen.playhead_map.column(take, on), Some(column));
            assert_eq!(screen.playhead_map.column(take, on + 0.01), Some(column));
        }
    }
}

#[test]
fn with_humanize_the_column_matches_scanning_every_column() {
    let mut screen = smf_screen_with_chords(12);
    press(&mut screen, 'd');
    let take = Take::Converted;
    let last = screen.column_on_seconds(11, take).unwrap();
    let mut seconds = -0.05;
    while seconds < last + 0.3 {
        assert_eq!(
            screen.playhead_map.column(take, seconds),
            scanned_column(&screen, take, seconds),
            "{seconds}"
        );
        seconds += 0.001;
    }
}

#[test]
fn toggling_a_rule_rebuilds_the_rows_for_the_new_events() {
    let mut screen = screen_with_mml("o3 l8 e g a b");
    let before: Vec<Option<usize>> = (0..screen.column_count())
        .map(|column| screen.playhead_map.row(Take::Converted, column))
        .collect();
    let events_before = screen.events(Take::Converted).len();

    press(&mut screen, 'l');
    press(&mut screen, 'v');

    assert_ne!(screen.events(Take::Converted).len(), events_before);
    let after: Vec<Option<usize>> = (0..screen.column_count())
        .map(|column| screen.playhead_map.row(Take::Converted, column))
        .collect();
    assert_ne!(after, before, "イベントが増えた分だけ行がずれる");
    assert_rows_match(&screen);
}
