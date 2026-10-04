use ratatui::{backend::TestBackend, Terminal};
use serde_json::json;

use super::*;

fn render(
    editor: &EffectChainEditor,
    catalog: Option<&AudioEffectCatalog>,
    adding: bool,
) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal
        .draw(|f| {
            let view = EffectChainView {
                adding,
                header: Line::from("instrument: METAL-GTX"),
                error: Some("試聴の準備に失敗しました"),
            };
            draw(f, f.area(), editor, catalog, view);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
        // 全角文字は TestBackend で後ろに空白が入るので、空白を落として比べる。
        .replace(' ', "")
}

#[test]
fn the_chain_list_shows_the_header_the_stages_and_the_error() {
    let catalog = crate::test_catalog::catalog();
    let editor = EffectChainEditor::open(vec![json!({"Test Amp preset": "Clean"})]);

    let screen = render(&editor, Some(&catalog), false);

    assert!(screen.contains("instrument:METAL-GTX"), "screen:\n{screen}");
    assert!(screen.contains("▶1.TestAmp:Clean"), "screen:\n{screen}");
    assert!(
        screen.contains("試聴の準備に失敗しました"),
        "screen:\n{screen}"
    );
}

#[test]
fn an_empty_chain_says_how_to_add_or_that_there_is_nothing_to_add() {
    let catalog = crate::test_catalog::catalog();
    let editor = EffectChainEditor::default();

    assert!(render(&editor, Some(&catalog), false).contains("(effectなし。aで追加)"));
    assert!(render(&editor, None, false).contains(&message::NO_PRESETS.replace(' ', "")));
}

#[test]
fn the_add_panes_list_presets_by_plugin_name() {
    let catalog = crate::test_catalog::catalog();
    let mut editor = EffectChainEditor::default();
    editor.open_add(&catalog, None);

    let screen = render(&editor, Some(&catalog), true);

    assert!(screen.contains("addpreset"), "screen:\n{screen}");
    assert!(screen.contains("AmpSimulator"), "screen:\n{screen}");
    assert!(screen.contains("TestAmpClean"), "screen:\n{screen}");
}

#[test]
fn a_long_stage_name_wraps_under_its_number_in_a_narrow_pane() {
    let catalog = crate::test_catalog::catalog();
    let editor = EffectChainEditor::open(vec![json!({"Test Amp preset": "Clean"})]);
    let mut terminal = Terminal::new(TestBackend::new(14, 5)).unwrap();
    terminal
        .draw(|f| {
            let block = Block::default().borders(Borders::ALL);
            draw_chain_pane(f, f.area(), block, &editor, Some(&catalog), "");
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let row = |y: u16| {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
    };

    // 枠の内側は 12 桁。`▶ 1. ` の 5 桁を引いた 7 桁に収まらない語は次の行へ送り、番号の後ろへ揃える。
    assert_eq!(row(1), "│▶ 1. Test   │");
    assert_eq!(row(2), "│     Amp:   │");
    assert_eq!(row(3), "│     Clean  │");
}

#[test]
fn wrap_words_splits_a_word_longer_than_the_row() {
    assert_eq!(wrap_words("Supermassive", 10, 3), ["Superma", "ssive"]);
    assert_eq!(wrap_words("A B", 10, 3), ["A B"]);
}
