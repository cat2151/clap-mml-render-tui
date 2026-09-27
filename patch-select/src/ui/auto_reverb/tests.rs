use cmrt_core::EffectPlugins;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, layout::Rect, text::Span, Terminal};

use crate::auto_reverb::tests::{test_catalog, DEXED_BASS, DEXED_SNARE, SURGE_PAD};
use crate::auto_reverb::AutoReverbRules;
use crate::ui::patch_select::{draw_in, PatchSelectDrawOptions};
use crate::{AutoReverbHost, PatchCatalogEntry, PatchSelect, PatchSelectRequest};

const SCREEN: Rect = Rect::new(0, 0, 120, 40);

fn patches() -> Vec<PatchCatalogEntry> {
    [DEXED_BASS, DEXED_SNARE]
        .into_iter()
        .map(|patch| PatchCatalogEntry::from_display(patch.to_string()).with_builtin_effects(false))
        .chain(std::iter::once(PatchCatalogEntry::from_display(
            SURGE_PAD.to_string(),
        )))
        .collect()
}

fn open(current: &str, auto_reverb: Option<AutoReverbHost>) -> PatchSelect<'static> {
    PatchSelect::open(PatchSelectRequest {
        patches: patches(),
        current: Some(current.to_string()),
        auto_reverb,
        ..Default::default()
    })
    .expect("patch list is not empty")
}

fn host(rules: AutoReverbRules, existing_chain: bool) -> Option<AutoReverbHost> {
    Some(AutoReverbHost {
        rules,
        effect_plugins: EffectPlugins::with_catalog(test_catalog()),
        existing_chain,
    })
}

/// 画面の文字列。全角文字の後ろの継続セルは読み飛ばし、見えるとおりの並びにする。
fn screen(select: &PatchSelect<'_>) -> Vec<String> {
    let mut terminal =
        Terminal::new(TestBackend::new(SCREEN.width, SCREEN.height)).expect("test terminal");
    terminal
        .draw(|frame| draw_in(select, frame, SCREEN, &PatchSelectDrawOptions::default()))
        .expect("draw");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            let mut line = String::new();
            let mut x = 0;
            while x < buffer.area.width {
                let symbol = buffer[(x, y)].symbol();
                line.push_str(symbol);
                x += (Span::raw(symbol).width() as u16).max(1);
            }
            line
        })
        .collect()
}

fn status_line(select: &PatchSelect<'_>) -> String {
    let lines = screen(select);
    lines
        .iter()
        .find(|line| line.starts_with("auto reverb: "))
        .unwrap_or_else(|| panic!("status line is not drawn:\n{}", lines.join("\n")))
        .trim_end()
        .to_string()
}

fn press(select: &mut PatchSelect<'_>, code: KeyCode) {
    select.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn the_status_line_names_the_applied_reverb_and_its_row() {
    let select = open(DEXED_SNARE, host(AutoReverbRules::default(), false));
    assert_eq!(
        status_line(&select),
        "auto reverb: Small Drum Room (snare)  e:ルール  E:on/off"
    );
}

#[test]
fn the_status_line_shows_a_dry_row() {
    let select = open(DEXED_BASS, host(AutoReverbRules::default(), false));
    assert_eq!(
        status_line(&select),
        "auto reverb: dry (bass|bs)  e:ルール  E:on/off"
    );
}

#[test]
fn the_status_line_shows_builtin_effects() {
    let select = open(SURGE_PAD, host(AutoReverbRules::default(), false));
    assert_eq!(
        status_line(&select),
        "auto reverb: -（音色に effect 内蔵）  e:ルール  E:on/off"
    );
}

#[test]
fn the_status_line_shows_that_the_existing_chain_wins() {
    let select = open(DEXED_SNARE, host(AutoReverbRules::default(), true));
    assert_eq!(
        status_line(&select),
        "auto reverb: -（track の effect を優先）  e:ルール  E:on/off"
    );
}

#[test]
fn the_status_line_shows_off() {
    let mut rules = AutoReverbRules::default();
    rules.set_enabled(false);
    let select = open(DEXED_SNARE, host(rules, false));
    assert_eq!(status_line(&select), "auto reverb: off  e:ルール  E:on/off");
}

#[test]
fn a_host_without_auto_reverb_draws_no_status_line() {
    let select = open(DEXED_SNARE, None);
    let lines = screen(&select);
    assert!(
        lines.iter().all(|line| !line.contains("auto reverb")),
        "{}",
        lines.join("\n")
    );
}

#[test]
fn the_rules_overlay_lists_rows_with_their_effects_and_the_effect_list_draws_on_top() {
    let mut select = open(DEXED_SNARE, host(AutoReverbRules::default(), false));
    press(&mut select, KeyCode::Char('e'));

    let lines = screen(&select);
    let text = lines.join("\n");
    assert!(text.contains("auto reverb ルール"), "{text}");
    let row = |name: &str| {
        lines
            .iter()
            .find(|line| line.contains(&format!("{name} ")) && line.contains('│'))
            .unwrap_or_else(|| panic!("row {name} is not drawn:\n{text}"))
            .clone()
    };
    assert!(row("bass|bs").contains("dry"), "{text}");
    assert!(row("snare").contains("Small Drum Room"), "{text}");

    press(&mut select, KeyCode::Char('x'));
    let text = screen(&select).join("\n");
    assert!(text.contains("Enter:決定"), "{text}");
    assert!(text.contains("なし(dry)"), "{text}");
    assert!(
        text.contains("Dragonfly Hall Reverb: Medium Clear Hall"),
        "{text}"
    );
}
