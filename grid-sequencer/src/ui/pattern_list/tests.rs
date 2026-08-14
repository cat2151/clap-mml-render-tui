use std::time::Instant;

use super::*;
use crate::{ChordPlayback, ListDirection};

/// section を全部出せる、十分な高さ。
const TALL: usize = 64;

/// arp / bass の section が出る画面（＝ chord mode 中）。drum 行は持たない。
fn chorded_screen() -> GridSequencerScreen {
    let mut screen = GridSequencerScreen::with_track_count(None, 3);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen
}

/// drum の section だけが出る画面（chord mode off）。
fn drum_screen() -> GridSequencerScreen {
    GridSequencerScreen::with_track_count(None, crate::FULL_DRUM_TRACK_COUNT)
}

fn texts(screen: &GridSequencerScreen, height: usize) -> Vec<String> {
    lines(screen, height)
        .into_iter()
        .map(|line| line.to_string())
        .collect()
}

fn column_texts(screen: &GridSequencerScreen, height: usize) -> Vec<Vec<String>> {
    let available = height.saturating_sub(BORDERS);
    section_columns(screen)
        .iter()
        .map(|sections| {
            lines_for_column(sections, available)
                .into_iter()
                .map(|line| line.to_string())
                .collect()
        })
        .collect()
}

fn arp_lines() -> usize {
    1 + ArpPattern::ALL.len()
}

fn bass_lines() -> usize {
    1 + BassPattern::ALL.len()
}

#[test]
fn both_sections_list_every_pattern_in_the_order_the_wheel_sends_them() {
    let screen = chorded_screen();
    let texts = texts(&screen, TALL);

    let expected = std::iter::once("arp".to_string())
        .chain(ArpPattern::ALL.iter().map(|p| format!("  {}", p.label())))
        .chain(std::iter::once("bass".to_string()))
        .chain(BassPattern::ALL.iter().map(|p| format!("  {}", p.label())))
        .collect::<Vec<_>>();
    assert_eq!(texts, expected);
}

fn drum_lines() -> usize {
    DrumRole::ALL
        .iter()
        .map(|role| 1 + DrumPattern::all_for(*role).len())
        .sum()
}

fn expected_drum_lines() -> Vec<String> {
    [
        DrumRole::Kick,
        DrumRole::Snare,
        DrumRole::HiHat,
        DrumRole::Percussion,
    ]
    .into_iter()
    .flat_map(|role| {
        std::iter::once(format!("drum {}", role.label().to_lowercase())).chain(
            DrumPattern::all_for(role)
                .into_iter()
                .map(|pattern| format!("  {}", pattern.label())),
        )
    })
    .collect()
}

/// chord mode を使わなくても、存在するdrum 4roleのlistはすべて出る。
#[test]
fn every_drum_section_appears_without_the_chord_mode() {
    let screen = drum_screen();

    assert!(screen.state.chord().is_none());
    assert_eq!(texts(&screen, TALL), expected_drum_lines());
}

/// 1候補しかないsnare / percussionもsectionごと省略しない。
#[test]
fn one_pattern_drum_roles_are_not_omitted() {
    let mut screen = drum_screen();
    screen.cycle_drum_pattern(
        crate::FIRST_DRUM_ROW,
        DrumRole::Percussion,
        ListDirection::Next,
    );

    let texts = texts(&screen, TALL);
    for expected in ["drum snr", "  Backbeat", "drum perc", "> Random"] {
        assert!(texts.iter().any(|line| line == expected), "{texts:?}");
    }
}

#[test]
fn every_drawn_drum_role_gets_its_own_marker() {
    let mut screen = drum_screen();
    let mut drawn = crate::DrawnPhrases::default();
    for pattern in cmrt_rhythm::DrumPatternCombination::all()[0].patterns() {
        drawn.record_drum(pattern);
    }
    screen.absorb_drawn_phrases(drawn);

    let marked = texts(&screen, TALL)
        .into_iter()
        .filter(|line| line.starts_with('>'))
        .collect::<Vec<_>>();

    assert_eq!(marked.len(), DrumRole::ALL.len());
    for role in DrumRole::ALL {
        let label = screen.last_drum_for(role).unwrap().label();
        assert!(marked.iter().any(|line| line == &format!("> {label}")));
    }
}

/// drum 行が無い構成では drum の section も出ない。
#[test]
fn a_grid_without_drum_rows_has_no_drum_section() {
    let screen = GridSequencerScreen::with_track_count(None, 3);

    assert!(texts(&screen, TALL).is_empty());
    assert!(section_heights(&screen).is_empty());
}

#[test]
fn nothing_is_marked_until_the_wheel_has_been_turned() {
    let screen = chorded_screen();

    assert!(!texts(&screen, TALL)
        .iter()
        .any(|line| line.starts_with('>')));
}

#[test]
fn the_marker_follows_the_pattern_the_wheel_last_applied() {
    let mut screen = chorded_screen();
    screen
        .state
        .display_drawn_now(crate::DrawnPhrases::with_arp(ArpPattern::UpDown));
    screen
        .state
        .display_drawn_now(crate::DrawnPhrases::with_bass(BassPattern::EighthOctave));

    let marked = texts(&screen, TALL)
        .into_iter()
        .filter(|line| line.starts_with('>'))
        .collect::<Vec<_>>();
    assert_eq!(marked, vec!["> UpDown", "> 8th+Oct"]);
}

/// 下へ回すと list の下の項目へ進む。並びが見えていることの意味はここにある。
#[test]
fn turning_the_wheel_down_moves_the_marker_down_the_list() {
    let mut screen = chorded_screen();
    screen
        .state
        .display_drawn_now(crate::DrawnPhrases::with_arp(ArpPattern::default()));
    let first = marked_index(&screen);

    screen
        .state
        .display_drawn_now(crate::DrawnPhrases::with_arp(ArpPattern::default().next()));

    assert_eq!(marked_index(&screen), first + 1);
}

#[test]
fn a_short_pane_clips_only_the_rows_below_its_height() {
    let screen = chorded_screen();

    let texts = texts(&screen, BORDERS + arp_lines() + bass_lines() - 1);
    assert_eq!(texts.len(), arp_lines() + bass_lines() - 1);
    assert_eq!(texts[0], "arp");
}

#[test]
fn a_short_drum_pane_keeps_all_drum_sections_ahead_of_chord_sections() {
    let mut screen = drum_screen();
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    let columns = column_texts(&screen, BORDERS + drum_lines());

    assert_eq!(columns.last().unwrap(), &expected_drum_lines());
}

/// 高さの見積もりと実際に描く行数が食い違うと、枠の中に空白が残る。
#[test]
fn the_pane_height_matches_the_lines_actually_drawn() {
    let screen = chorded_screen();

    for available in 0..=(BORDERS + arp_lines() + bass_lines() + 4) {
        let height = height_for(
            &section_heights(&screen),
            u16::try_from(available).expect("small"),
        );
        let drawn = lines(&screen, usize::from(height)).len();
        assert_eq!(
            usize::from(height),
            if drawn == 0 { 0 } else { BORDERS + drawn },
            "available={available}"
        );
    }
}

/// pane の幅は一番長いラベルと印がちょうど収まる幅。溢れると右端が切れる。
#[test]
fn every_entry_fits_the_pane_width() {
    let content = (usize::from(crate::ui::layout::PATTERN_LIST_WIDTH) - BORDERS) / 2;

    for screen in [chorded_screen(), drum_screen()] {
        for line in texts(&screen, TALL) {
            assert!(line.chars().count() <= content, "{line}");
        }
    }
}

/// drum は役割ごとに list が違うので、どの役割でも幅に収まることを見る。
#[test]
fn every_drum_role_entry_fits_the_pane_width() {
    let content = (usize::from(crate::ui::layout::PATTERN_LIST_WIDTH) - BORDERS) / 2;

    for role in DrumRole::ALL {
        assert!(
            format!("drum {}", role.label().to_lowercase())
                .chars()
                .count()
                <= content
        );
        for pattern in DrumPattern::all_for(role) {
            assert!(
                pattern.label().chars().count() + 2 <= content,
                "{}",
                pattern.label()
            );
        }
    }
}

fn marked_index(screen: &GridSequencerScreen) -> usize {
    texts(screen, TALL)
        .iter()
        .position(|line| line.starts_with('>'))
        .expect("印が付いている")
}

/// wheel からの実際の送りでも、印が list の並びどおりに動く。
#[test]
fn the_marker_tracks_a_real_wheel_turn() {
    let mut screen = chorded_screen();
    screen.cycle_arpeggio(0, ListDirection::Next);
    assert_eq!(screen.last_arp(), None, "voice が足りない行では動かない");

    screen
        .state
        .display_drawn_now(crate::DrawnPhrases::with_arp(ArpPattern::default()));
    assert_eq!(marked_index(&screen), 1, "arp 見出しの次が先頭の型");
}
