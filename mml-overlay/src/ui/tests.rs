use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

use super::*;
use crate::MmlOverlayContext;

fn render(overlay: &MmlOverlay<'_>) -> String {
    let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
    terminal.draw(|frame| draw(overlay, frame)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn opened() -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext::default());
    overlay
}

fn overlay_with(input: &str) -> MmlOverlay<'static> {
    let mut overlay = opened();
    let now = Instant::now();
    for code in input.chars().map(KeyCode::Char) {
        overlay.handle_key(KeyEvent::new(code, KeyModifiers::NONE), now);
    }
    overlay
}

#[test]
fn draws_the_title_the_typed_mml_and_the_key_hints() {
    let rendered = render(&overlay_with("cde"));

    assert!(rendered.contains("MML"), "{rendered}");
    assert!(rendered.contains("cde"), "{rendered}");
    // 全角文字はセル単位で分かれて見えるので、ASCII の部分だけで確かめる。
    assert!(rendered.contains("Esc"), "{rendered}");
}

#[test]
fn shows_the_sounding_note_name() {
    let rendered = render(&overlay_with("c"));

    assert!(rendered.contains("c5"), "{rendered}");
}

#[test]
fn shows_every_member_of_a_sounding_chord() {
    let rendered = render(&overlay_with("'ceg'"));

    assert!(rendered.contains("c5 e5 g5"), "{rendered}");
}

/// 打鍵の音がコード表記から来たかどうかも、その場で分かるようにする。
#[test]
fn shows_that_a_typed_chord_name_was_read_as_a_chord() {
    let rendered = render(&overlay_with("C"));

    assert!(rendered.contains("CHORD"), "{rendered}");
    assert!(rendered.contains("c5 e5 g5"), "{rendered}");
}

#[test]
fn shows_the_default_patch_when_none_is_chosen() {
    let rendered = render(&opened());

    assert!(rendered.contains("MML"), "{rendered}");
}

#[test]
fn shows_the_selected_patch_in_the_title() {
    let mut overlay = MmlOverlay::default();
    overlay.set_restored_patch(Some("Leads/Lead 1.fxp".to_string()));
    overlay.open(MmlOverlayContext::default());
    let rendered = render(&overlay);

    assert!(rendered.contains("Lead 1.fxp"), "{rendered}");
}

/// 行を演奏したら、コードとして読まれたのか MML として読まれたのかを出す。
#[test]
fn shows_whether_the_played_line_was_read_as_a_chord() {
    let mut overlay = opened();
    let now = Instant::now();
    for code in "C".chars().map(KeyCode::Char) {
        overlay.handle_key(KeyEvent::new(code, KeyModifiers::NONE), now);
    }
    overlay.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), now);
    overlay.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), now);

    assert!(render(&overlay).contains("CHORD"), "{}", render(&overlay));
}

#[test]
fn draws_the_patch_select_over_the_input() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patches: vec![(
            "Leads/Lead 1.fxp".to_string(),
            "leads/lead 1.fxp".to_string(),
        )],
        ..MmlOverlayContext::default()
    });
    overlay.handle_key(
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
        Instant::now(),
    );
    let rendered = render(&overlay);

    assert!(rendered.contains("Enter:"), "{rendered}");
    assert!(rendered.contains("Lead 1.fxp"), "{rendered}");
}

#[test]
fn draws_the_history_select_over_the_input() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        history: vec!["cdefg".to_string()],
        favorites: vec!["gfedc".to_string()],
        ..MmlOverlayContext::default()
    });
    overlay.handle_key(
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
        Instant::now(),
    );
    let rendered = render(&overlay);

    assert!(rendered.contains("cdefg"), "{rendered}");
    assert!(rendered.contains("gfedc"), "{rendered}");
}

/// カタログから外れたプラグインの案内は、音色選択を開いている間だけ枠の下に出る。
///
/// 一覧に**出てこない**ものの話なので、一覧をいくら眺めても分からない。
/// これが出ないと「Vaporizer2 を入れたのに 1 件も出ない」の原因に辿り着けない。
#[test]
fn the_patch_select_shows_why_a_plugin_is_missing_from_the_list() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patches: vec![(
            "Leads/Lead 1.fxp".to_string(),
            "leads/lead 1.fxp".to_string(),
        )],
        catalog_notes: vec!["Vaporizer2 は patches_dirs が無いため一覧に出ません".to_string()],
        ..MmlOverlayContext::default()
    });
    overlay.handle_key(
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
        Instant::now(),
    );

    let rendered = render(&overlay);

    assert!(rendered.contains("Vaporizer2"), "{rendered}");
    assert!(rendered.contains("patches_dirs"), "{rendered}");
}

/// 案内が無いときは 1 行も取らない。ふだんの見え方を変えないことの番人。
#[test]
fn the_patch_select_takes_no_extra_row_without_a_note() {
    let patches = vec![(
        "Leads/Lead 1.fxp".to_string(),
        "leads/lead 1.fxp".to_string(),
    )];
    let open_with = |catalog_notes: Vec<String>| {
        let mut overlay = MmlOverlay::default();
        overlay.open(MmlOverlayContext {
            patches: patches.clone(),
            catalog_notes,
            ..MmlOverlayContext::default()
        });
        overlay.handle_key(
            KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
            Instant::now(),
        );
        render(&overlay)
    };

    let without = open_with(Vec::new());
    let with = open_with(vec![
        "Vaporizer2 は patches_dirs が無いため一覧に出ません".to_string()
    ]);

    assert_ne!(without, with);
    // 案内が無いほうには、案内のための空行も色も出ない。
    assert!(!without.contains("Vaporizer2"), "{without}");
}
