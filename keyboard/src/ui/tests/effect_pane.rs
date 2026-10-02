use super::*;
use cmrt_core::EffectPlugins;
use cmrt_effect_chain_select::messages;
use cmrt_tui_core::theme::MONOKAI_FG;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::effect_pane::tests::catalog;

struct NoVoicing;

impl crate::KeyboardVoicingLookup for NoVoicing {
    fn cached_voicing(&self, _patch: &str) -> Option<cmrt_realtime_play::PatchVoicing> {
        None
    }
}

fn patch_load() -> cmrt_tui_core::patch_load::PatchLoadState {
    cmrt_tui_core::patch_load::PatchLoadState::ready(
        ["Leads/Saw Lead.fxp", "Pads/Warm Pad.fxp"]
            .iter()
            .map(|name| (name.to_string(), name.to_lowercase()))
            .collect(),
    )
}

fn screen(
    effect_plugins: EffectPlugins,
    chain: Vec<serde_json::Value>,
    load: &cmrt_tui_core::patch_load::PatchLoadState,
) -> crate::KeyboardScreen<'static> {
    let ctx = context(load);
    let mut screen = crate::KeyboardScreen::new(
        None,
        KeyboardState::new(Some("Leads/Saw Lead.fxp".to_string())),
        crate::KeyboardMmlInput::default(),
        crate::KeyboardNoteGuide::new(None),
    )
    .with_effect_plugins(effect_plugins)
    .with_effect_chain(chain);
    screen.sync_patch_catalog(&ctx);
    screen
}

fn context(load: &cmrt_tui_core::patch_load::PatchLoadState) -> crate::KeyboardContext<'_> {
    crate::KeyboardContext {
        patch_dirs_configured: true,
        patch_load: load,
        voicing: &NoVoicing,
        catalog_notes: &[],
    }
}

fn render(screen: &mut crate::KeyboardScreen<'_>, width: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, 30)).unwrap();
    terminal
        .draw(|f| draw(screen, &crate::KeyboardConnectionStatus::default(), f))
        .unwrap();
    terminal
}

/// 1 行目で pane の左上の角がある桁。左から t の欄・Role・Preset・Patches・Effect。
fn pane_corners(terminal: &Terminal<TestBackend>) -> Vec<u16> {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.width)
        .filter(|&x| buffer.cell((x, 0)).unwrap().symbol() == "┌")
        .collect()
}

#[test]
fn the_effect_pane_sits_at_the_right_end_and_turns_yellow_when_focused() {
    let load = patch_load();
    let ctx = context(&load);
    let stages: Vec<_> = catalog()
        .presets()
        .iter()
        .map(|preset| preset.json_element())
        .collect();
    let mut screen = screen(EffectPlugins::with_catalog(catalog()), stages, &load);
    screen.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE), &ctx);

    let terminal = render(&mut screen, 160);
    let text = buffer_to_string(&terminal);
    assert!(text.contains(" Effect (2) "), "{text}");
    assert!(text.contains("1. "), "{text}");
    assert!(text.contains("2. "), "{text}");
    assert!(
        text.contains("a:add  r:replace  dd:del  b:bypass"),
        "{text}"
    );

    let corners = pane_corners(&terminal);
    assert_eq!(corners.len(), 5, "{corners:?}\n{text}");
    let (patches_x, effect_x) = (corners[3], corners[4]);
    assert!(effect_x - patches_x >= 12, "{corners:?}");
    // Effect pane は右端まで。
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.cell((159, 0)).unwrap().symbol(), "┐");
    assert_eq!(buffer.cell((effect_x, 0)).unwrap().fg, MONOKAI_YELLOW);
    assert_eq!(buffer.cell((patches_x, 0)).unwrap().fg, MONOKAI_FG);
}

#[test]
fn an_empty_pane_without_a_catalog_wraps_the_notice() {
    let load = patch_load();
    let mut screen = screen(EffectPlugins::none(), Vec::new(), &load);

    let text = buffer_to_string(&render(&mut screen, 160));

    assert!(text.contains(" Effect (0) "), "{text}");
    // 狭い pane では折り返す。先頭と末尾の語がどちらも見えていれば切れていない。
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(compact.contains("effect"), "{text}");
    assert!(compact.contains("使えない"), "{text}");
    assert!(
        !text.contains(messages::EMPTY_CHAIN),
        "an unavailable backend must not invite `a`\n{text}"
    );
}

#[test]
fn the_add_overlay_is_drawn_over_the_panes() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen(EffectPlugins::with_catalog(catalog()), Vec::new(), &load);
    for ch in ['l', 'a'] {
        screen.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE), &ctx);
    }

    let text = buffer_to_string(&render(&mut screen, 160));

    assert!(text.contains(messages::ADD_OVERLAY_TITLE), "{text}");
    assert!(text.contains("keyboard: Leads/Saw Lead.fxp"), "{text}");
    assert!(text.contains("Delay/Echo"), "{text}");
}
