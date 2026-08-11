use super::*;
use crate::ListDirection;

/// 枠線ぶんを足した、両方の section が入りきる高さ。
const TALL: usize = BORDERS + ARP_LINES + BASS_LINES;

fn texts(screen: &GridSequencerScreen, height: usize) -> Vec<String> {
    lines(screen, height)
        .into_iter()
        .map(|line| line.to_string())
        .collect()
}

#[test]
fn both_sections_list_every_pattern_in_the_order_the_wheel_sends_them() {
    let screen = GridSequencerScreen::new(None);
    let texts = texts(&screen, TALL);

    let expected = std::iter::once("arp".to_string())
        .chain(ArpPattern::ALL.iter().map(|p| format!("  {}", p.label())))
        .chain(std::iter::once("bass".to_string()))
        .chain(BassPattern::ALL.iter().map(|p| format!("  {}", p.label())))
        .collect::<Vec<_>>();
    assert_eq!(texts, expected);
}

#[test]
fn nothing_is_marked_until_the_wheel_has_been_turned() {
    let screen = GridSequencerScreen::new(None);

    assert!(!texts(&screen, TALL)
        .iter()
        .any(|line| line.starts_with('>')));
}

#[test]
fn the_marker_follows_the_pattern_the_wheel_last_applied() {
    let mut screen = GridSequencerScreen::new(None);
    screen.last_arp = Some(ArpPattern::UpDown);
    screen.last_bass = Some(BassPattern::EighthOctave);

    let marked = texts(&screen, TALL)
        .into_iter()
        .filter(|line| line.starts_with('>'))
        .collect::<Vec<_>>();
    assert_eq!(marked, vec!["> UpDown", "> 8th+Oct"]);
}

/// 下へ回すと list の下の項目へ進む。並びが見えていることの意味はここにある。
#[test]
fn turning_the_wheel_down_moves_the_marker_down_the_list() {
    let mut screen = GridSequencerScreen::new(None);
    screen.last_arp = Some(ArpPattern::default());
    let first = marked_index(&screen);

    screen.last_arp = Some(ArpPattern::default().next());

    assert_eq!(marked_index(&screen), first + 1);
}

#[test]
fn a_short_pane_drops_the_bass_section_before_it_clips_the_arp_one() {
    let screen = GridSequencerScreen::new(None);

    let texts = texts(&screen, TALL - 1);
    assert_eq!(texts.len(), ARP_LINES);
    assert_eq!(texts[0], "arp");
}

/// pane の幅は一番長いラベルと印がちょうど収まる幅。溢れると右端が切れる。
#[test]
fn every_entry_fits_the_pane_width() {
    let screen = GridSequencerScreen::new(None);
    let content = usize::from(crate::ui::layout::PATTERN_LIST_WIDTH) - BORDERS;

    for line in texts(&screen, TALL) {
        assert!(line.chars().count() <= content, "{line}");
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
    let mut screen = GridSequencerScreen::new(None);
    screen.cycle_arpeggio(0, ListDirection::Next);
    assert_eq!(screen.last_arp(), None, "voice が足りない行では動かない");

    screen.last_arp = Some(ArpPattern::default());
    assert_eq!(marked_index(&screen), 1, "arp 見出しの次が先頭の型");
}
