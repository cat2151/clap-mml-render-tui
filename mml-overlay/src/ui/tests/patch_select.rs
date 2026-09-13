use super::*;

#[test]
fn filter_editing_shows_its_own_enter_and_escape_actions() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patch_catalog: PatchCatalogSnapshot::Ready(vec![PatchCatalogEntry::from_display(
            "Leads/Lead 1.fxp".to_string(),
        )]),
        ..MmlOverlayContext::default()
    });
    let now = Instant::now();
    overlay.handle_key(
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
        now,
    );
    overlay.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE), now);

    let rendered = render(&overlay).replace(' ', "");

    assert!(rendered.contains("Enter:絞り込み確定"), "{rendered}");
    assert!(rendered.contains("Esc:前回へ戻す"), "{rendered}");
}
