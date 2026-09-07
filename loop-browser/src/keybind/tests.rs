use super::*;

#[test]
fn the_tree_keybind_line_advertises_the_filter_key() {
    let text = loop_browser_keybind_text(LoopBrowserPane::Tree);

    assert!(text.contains("/:絞り込み"), "{text}");
}

#[test]
fn the_tracks_keybind_line_has_no_filter_key() {
    let text = loop_browser_keybind_text(LoopBrowserPane::Tracks);

    assert!(!text.contains("絞り込み"), "{text}");
}
