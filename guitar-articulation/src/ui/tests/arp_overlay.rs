//! アルペジエーター overlay（`z`）の param pane・素材 pane・キーの行・ヘルプ（`?`）。

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

/// overlay の中で `label` を含む最初の行（空白を詰めたもの）と、その行の y。
fn find_row(buffer: &Buffer, label: &str) -> (usize, String) {
    let rows = rows_in(buffer, buffer.area);
    let top = overlay_top(&rows);
    rows.iter()
        .enumerate()
        .skip(top)
        .map(|(y, row)| (y, squeezed(std::slice::from_ref(row))))
        .find(|(_, row)| row.contains(label))
        .unwrap_or_else(|| panic!("{label} の行が無い: {rows:#?}"))
}

/// overlay の中で `label` の行の先頭に反転（太字）が掛かっているか。
fn is_highlighted(buffer: &Buffer, label: &str) -> bool {
    let rows = rows_in(buffer, buffer.area);
    let (y, _) = find_row(buffer, label);
    // 全角の 2 セル目が空白で読めるので、行の中は先頭の 1 文字で探す。
    let first = label.chars().next().unwrap();
    let x = rows[y][..rows[y].find(first).unwrap()].chars().count() as u16;
    buffer
        .cell((x, y as u16))
        .unwrap()
        .modifier
        .contains(Modifier::BOLD)
}

fn open(mml: &str) -> GuitarArticulationScreen {
    let mut screen = screen_with_mml(mml);
    screen.handle_key_event(key(KeyCode::Char('z')));
    screen
}

#[test]
fn the_param_pane_shows_the_rows_and_highlights_the_selected_one() {
    let mut screen = open("l16cdef");
    let buffer = render(&screen);
    let all = squeezed(&overlay_rows(&buffer));
    for row in ["素材-/0l16cdef", "音型Up", "oct1", "シフト0"] {
        assert!(all.contains(row), "{row}\n{all}");
    }
    assert!(is_highlighted(&buffer, "素材"));
    assert!(!is_highlighted(&buffer, "音型"));

    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    let buffer = render(&screen);
    assert!(is_highlighted(&buffer, "音型"));
    let all = squeezed(&overlay_rows(&buffer));
    assert!(all.contains("音型Down"), "{all}");
    assert!(!all.contains("ON/OFF"), "{all}");
}

#[test]
fn the_down_row_shows_only_for_up_down_and_the_tempo_rows_only_for_a_chord() {
    let mut mml = open("l16cdef");
    let all = squeezed(&overlay_rows(&render(&mml)));
    // 最下行のキーにも `BPM` があるので、値まで含めて探す。
    for hidden in ["下り幅", "BPM120", "音価"] {
        assert!(!all.contains(hidden), "{hidden}\n{all}");
    }
    mml.set_arp(crate::ArpSettings {
        pattern: cmrt_arpeggiator::ArpPattern::UpDown,
        ..crate::ArpSettings::default()
    });
    let all = squeezed(&overlay_rows(&render(&mml)));
    assert!(all.contains("下り幅全部"), "{all}");
    assert!(!all.contains("BPM120"), "{all}");

    let chord = open("Am7");
    let all = squeezed(&overlay_rows(&render(&chord)));
    assert!(all.contains("BPM120"), "{all}");
    assert!(all.contains("音価16分"), "{all}");
}

#[test]
fn the_material_pane_lists_the_materials_and_marks_the_current_one() {
    let empty = open("l16cdef");
    let all = squeezed(&overlay_rows(&render(&empty)));
    assert!(all.contains("(Tabで追加)"), "{all}");

    let mut chord = screen_with_mml("Am7")
        .with_arp_materials(["C", "Am7", "l16cdef"].map(String::from).to_vec());
    chord.handle_key_event(key(KeyCode::Char('z')));
    let all = squeezed(&overlay_rows(&render(&chord)));
    assert!(all.contains("素材2/3Am7"), "{all}");
    assert!(all.contains("▶Am7"), "{all}");
    assert!(!all.contains("▶C"), "{all}");
}

#[test]
fn the_material_pane_shows_the_text_while_editing() {
    let mut screen = open("l16cdef");
    screen.handle_key_event(key(KeyCode::Tab));
    for ch in "Am7".chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
    let buffer = render(&screen);
    let all = squeezed(&overlay_rows(&buffer));
    assert!(all.contains("Am7"), "{all}");
    assert!(all.contains("Tab/Esc:確定Enter:改行"), "{all}");
    assert!(
        !is_highlighted(&buffer, "素材"),
        "編集中は param の行を反転しない"
    );
}

#[test]
fn the_overlay_names_only_the_keys_and_the_help_explains_the_rows() {
    let mut screen = open("l16cdef");
    let all = squeezed(&overlay_rows(&render(&screen)));
    assert!(
        all.contains("j/k:行h/l:値H/L:BPM±1Tab:素材space:演奏Esc:閉じる?:help"),
        "{all}"
    );
    for removed in ["UpTurn", "UpDownHold", "回数", "隙間なく"] {
        assert!(!all.contains(removed), "{removed}\n{all}");
    }

    screen.handle_key_event(key(KeyCode::Char('?')));
    let all = squeezed(&rows_in(&render(&screen), Rect::new(0, 0, WIDTH, HEIGHT)));
    for help in [
        "アルペジエーターヘルプ",
        "UpDownで最高音から下りる音数",
        "1行1素材",
        "閉じるとMML欄のMMLで鳴る",
        "列ごとのルールは当てない",
    ] {
        assert!(all.contains(help), "{help}\n{all}");
    }
    assert!(!all.contains("MML欄の素材"), "{all}");
}

#[test]
fn the_footer_shows_an_error_in_pink() {
    let mut screen = open("l16cdef");
    screen.handle_key_event(key(KeyCode::Tab));
    for ch in "@@@".chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
    screen.handle_key_event(key(KeyCode::Tab));
    let buffer = render(&screen);
    let (y, row) = find_row(&buffer, "!");
    assert!(!row.contains("j/k:行"), "{row}");
    let rows = rows_in(&buffer, buffer.area);
    let x = rows[y][..rows[y].find('!').unwrap()].chars().count() as u16;
    assert_eq!(
        buffer.cell((x, y as u16)).unwrap().fg,
        cmrt_tui_core::theme::MONOKAI_PINK
    );
}

#[test]
fn the_shared_rule_rows_show_their_names_and_values() {
    let mut screen = open("l16cdef");
    let all = squeezed(&overlay_rows(&render(&screen)));
    for row in ["汚し汚しなし", "奏法ピッキング", "アクセント上"] {
        assert!(
            all.contains(row),
            "{row}
{all}"
        );
    }

    for _ in 0..screen.arp_rows().len() {
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    for _ in 0..2 {
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
    screen.handle_key_event(key(KeyCode::Char('k')));
    for _ in 0..3 {
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
    screen.handle_key_event(key(KeyCode::Char('k')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    let buffer = render(&screen);
    assert!(is_highlighted(&buffer, "汚し"));
    let all = squeezed(&overlay_rows(&buffer));
    for row in [
        "汚し汚し&汚しrelease",
        "奏法オートプリング2",
        "アクセント上下",
    ] {
        assert!(
            all.contains(row),
            "{row}
{all}"
        );
    }

    screen.handle_key_event(key(KeyCode::Char('?')));
    let all = squeezed(&rows_in(&render(&screen), Rect::new(0, 0, WIDTH, HEIGHT)));
    for help in [
        "汚しなし/汚し&汚しrelease",
        "ピッキング/エコ/オートプリング1",
        "アクセント上/下/上下",
    ] {
        assert!(
            all.contains(help),
            "{help}
{all}"
        );
    }
}
