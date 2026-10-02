//! アルペジエーター overlay（`z`）に音型の一覧・選択中・設定・step 数が出る。

use ratatui::style::Modifier;

use super::*;

/// overlay の枠の行。全角の 2 セル目は空白で読めるので、空白を落として探す。
fn overlay_top(rows: &[String]) -> usize {
    rows.iter()
        .position(|row| squeezed(std::slice::from_ref(row)).contains("アルペジエーター"))
        .unwrap_or_else(|| panic!("overlay が無い: {rows:#?}"))
}

/// overlay の枠から下の行。
fn overlay_rows(buffer: &Buffer) -> Vec<String> {
    let rows = rows_in(buffer, buffer.area);
    rows[overlay_top(&rows)..].to_vec()
}

/// overlay の中で `label` の行に反転（太字）が掛かっているか。
fn is_highlighted(buffer: &Buffer, label: &str) -> bool {
    let rows = rows_in(buffer, buffer.area);
    let top = overlay_top(&rows);
    let y = top
        + rows[top..]
            .iter()
            .position(|row| row.contains(label))
            .unwrap();
    let x = rows[y][..rows[y].find(label).unwrap()].chars().count() as u16;
    buffer
        .cell((x, y as u16))
        .unwrap()
        .modifier
        .contains(Modifier::BOLD)
}

#[test]
fn the_overlay_lists_the_patterns_and_shows_off_until_a_pattern_is_picked() {
    let mut screen = screen_with_mml("l16cdef");
    screen.handle_key_event(key(KeyCode::Char('z')));
    let buffer = render(&screen);
    let rows = overlay_rows(&buffer);
    let all = squeezed(&rows);

    for (key, pattern) in crate::ui::ARP_PATTERN_KEYS {
        assert!(
            all.contains(&format!("{key}{}", pattern.label())),
            "{key} {pattern:?}\n{all}"
        );
    }
    assert!(all.contains("OFF(oct1回数2戻り幅2)"), "{all}");
    assert!(!is_highlighted(&buffer, "Up"), "OFF の間は反転しない");
}

#[test]
fn the_overlay_highlights_the_picked_pattern_and_shows_the_step_count() {
    let mut screen = screen_with_mml("l16cdef");
    screen.handle_key_event(key(KeyCode::Char('z')));
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    screen.handle_key_event(key(KeyCode::Char('2')));
    screen.handle_key_event(key(KeyCode::Char('b')));
    let buffer = render(&screen);
    let all = squeezed(&overlay_rows(&buffer));

    assert!(is_highlighted(&buffer, "UpTurn"), "{all}");
    assert!(!is_highlighted(&buffer, "UpDown"), "{all}");
    let steps = screen.column_count();
    assert!(
        all.contains(&format!("oct2回数2戻り幅3step{steps}")),
        "{all}"
    );
    // 見出しの `0:OFF` ではなく、最後の行の OFF 表示が消えている。
    assert!(!all.contains("OFF(oct"), "{all}");
}

#[test]
fn the_overlay_offers_only_the_screen_patterns_and_explains_its_keys() {
    let patterns: Vec<_> = crate::ui::ARP_PATTERN_KEYS
        .iter()
        .map(|(_, pattern)| *pattern)
        .collect();
    assert_eq!(patterns, crate::ARP_PATTERNS);

    let mut screen = screen_with_mml("l16cdef");
    screen.handle_key_event(key(KeyCode::Char('z')));
    let rows = overlay_rows(&render(&screen));
    let all = squeezed(&rows);
    for label in ["Converge", "Diverge", "Octave", "Random"] {
        assert!(!all.contains(label), "{label}\n{all}");
    }
    for (keys, help) in [
        (" 1 / 2 / 3 ", "oct を 1 / 2 / 3 にする"),
        (" n / N ", "回数を +1 / -1"),
        (" b / B ", "戻り幅を +1 / -1"),
    ] {
        // キーと説明の間を空白で空ける。全角の 2 セル目も空白なので、説明は空白を詰めて探す。
        let row = rows
            .iter()
            .find(|row| row.contains(keys))
            .unwrap_or_else(|| panic!("{keys}\n{all}"));
        let after = &row[row.find(keys).unwrap() + keys.len()..];
        assert!(after.starts_with("  "), "{row}");
        assert!(
            squeezed(std::slice::from_ref(row)).contains(&help.replace(' ', "")),
            "{row}"
        );
    }
}
