use super::*;

use cmrt_core::EffectPlugins;
use serde_json::json;

use crate::auto_reverb::tests::{
    test_catalog, DEXED_BASS, DEXED_KICK, DEXED_PAD, DEXED_SNARE, SURGE_PAD,
};
use crate::auto_reverb::AutoReverbRules;

pub(crate) fn auto_reverb_patches() -> Vec<PatchCatalogEntry> {
    [DEXED_BASS, DEXED_KICK, DEXED_SNARE, DEXED_PAD]
        .into_iter()
        .map(|patch| PatchCatalogEntry::from_display(patch.to_string()).with_builtin_effects(false))
        .chain(std::iter::once(PatchCatalogEntry::from_display(
            SURGE_PAD.to_string(),
        )))
        .collect()
}

pub(crate) fn open_with_auto_reverb(
    current: &str,
    rules: AutoReverbRules,
    existing_chain: bool,
) -> PatchSelect<'static> {
    PatchSelect::open(PatchSelectRequest {
        patches: auto_reverb_patches(),
        current: Some(current.to_string()),
        auto_reverb: Some(AutoReverbHost {
            rules,
            effect_plugins: EffectPlugins::with_catalog(test_catalog()),
            existing_chain,
        }),
        ..Default::default()
    })
    .expect("patch list is not empty")
}

fn shift(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

fn row_index(select: &PatchSelect<'_>, name: &str) -> usize {
    select
        .auto_reverb_rules()
        .unwrap()
        .rows()
        .iter()
        .position(|(row, _)| row.name() == name)
        .unwrap()
}

fn move_overlay_to(select: &mut PatchSelect<'_>, name: &str) {
    let target = row_index(select, name);
    while select.auto_reverb_overlay().unwrap().cursor() < target {
        select.handle_key(press(KeyCode::Char('j')));
    }
    assert_eq!(select.auto_reverb_overlay().unwrap().cursor(), target);
}

fn effect_list_labels(select: &PatchSelect<'_>) -> Vec<String> {
    select
        .auto_reverb_overlay()
        .unwrap()
        .effect_list()
        .unwrap()
        .choices()
        .iter()
        .map(|choice| choice.label.clone())
        .collect()
}

fn row_effect(select: &PatchSelect<'_>, name: &str) -> Option<serde_json::Value> {
    select.auto_reverb_rules().unwrap().rows()[row_index(select, name)]
        .1
        .clone()
}

#[test]
fn a_host_without_auto_reverb_ignores_e_and_shift_e() {
    let mut select = PatchSelect::open(PatchSelectRequest {
        patches: auto_reverb_patches(),
        current: Some(DEXED_SNARE.to_string()),
        ..Default::default()
    })
    .unwrap();

    assert_eq!(
        select.handle_key(press(KeyCode::Char('e'))),
        PatchSelectAction::Continue
    );
    assert_eq!(select.handle_key(shift('E')), PatchSelectAction::Continue);
    assert!(!select.auto_reverb_overlay_open());
    assert_eq!(select.auto_reverb_status(), None);
    assert_eq!(select.auto_reverb_stage(DEXED_SNARE), None);
}

#[test]
fn e_opens_the_rules_overlay_and_it_captures_every_key() {
    let mut select = open_with_auto_reverb(DEXED_SNARE, AutoReverbRules::default(), false);

    select.handle_key(press(KeyCode::Char('e')));

    assert!(select.auto_reverb_overlay_open());
    assert!(select.captures_all_keys());
    // overlay の j は音色カーソルではなく行を動かす。
    select.handle_key(press(KeyCode::Char('j')));
    assert_eq!(select.selected(), Some(DEXED_SNARE));
    assert_eq!(select.auto_reverb_overlay().unwrap().cursor(), 1);
    select.handle_key(press(KeyCode::Char('k')));
    select.handle_key(press(KeyCode::Up));
    assert_eq!(select.auto_reverb_overlay().unwrap().cursor(), 0);
    // overlay の Enter は音色を確定しない。
    assert_eq!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Continue
    );
}

#[test]
fn x_lists_dry_first_then_selectable_reverbs_only() {
    let mut select = open_with_auto_reverb(DEXED_SNARE, AutoReverbRules::default(), false);
    select.handle_key(press(KeyCode::Char('e')));
    move_overlay_to(&mut select, "snare");

    select.handle_key(press(KeyCode::Char('x')));

    assert_eq!(
        effect_list_labels(&select),
        [
            "なし(dry)",
            "Dragonfly Room Reverb: Small Drum Room",
            "Dragonfly Room Reverb: Large Drum Room",
            "Dragonfly Hall Reverb: Medium Clear Hall",
            "Surge XT Effects: Reverb 2/Room",
        ]
    );
    // 行の今の effect（snare の既定 Small Drum Room）から始まる。
    assert_eq!(
        select
            .auto_reverb_overlay()
            .unwrap()
            .effect_list()
            .unwrap()
            .cursor(),
        1
    );
}

#[test]
fn enter_in_the_effect_list_sets_the_row_and_esc_saves_and_replays() {
    let mut select = open_with_auto_reverb(DEXED_SNARE, AutoReverbRules::default(), false);
    select.handle_key(press(KeyCode::Char('e')));
    move_overlay_to(&mut select, "snare");
    select.handle_key(press(KeyCode::Char('x')));
    select.handle_key(press(KeyCode::Char('j')));

    assert_eq!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Continue
    );

    let large = json!({"Dragonfly Room Reverb preset": "Large Drum Room"});
    assert_eq!(row_effect(&select, "snare"), Some(large.clone()));
    assert!(select
        .auto_reverb_overlay()
        .unwrap()
        .effect_list()
        .is_none());
    assert_eq!(select.auto_reverb_stage(DEXED_SNARE), Some(large.clone()));

    let PatchSelectAction::SaveAutoReverb { rules, preview } =
        select.handle_key(press(KeyCode::Esc))
    else {
        panic!("closing a changed overlay must ask the host to save");
    };
    assert_eq!(preview.as_deref(), Some(DEXED_SNARE));
    assert_eq!(&rules, select.auto_reverb_rules().unwrap());
    assert!(!select.auto_reverb_overlay_open());
}

#[test]
fn choosing_dry_clears_the_row() {
    let mut select = open_with_auto_reverb(DEXED_PAD, AutoReverbRules::default(), false);
    select.handle_key(press(KeyCode::Char('e')));
    move_overlay_to(&mut select, "pad");
    select.handle_key(press(KeyCode::Char('x')));
    while select
        .auto_reverb_overlay()
        .unwrap()
        .effect_list()
        .unwrap()
        .cursor()
        > 0
    {
        select.handle_key(press(KeyCode::Char('k')));
    }
    select.handle_key(press(KeyCode::Enter));

    assert_eq!(row_effect(&select, "pad"), None);
    assert_eq!(select.auto_reverb_stage(DEXED_PAD), None);
}

#[test]
fn esc_in_the_effect_list_goes_back_without_changing_the_row() {
    let mut select = open_with_auto_reverb(DEXED_SNARE, AutoReverbRules::default(), false);
    select.handle_key(press(KeyCode::Char('e')));
    move_overlay_to(&mut select, "snare");
    let before = row_effect(&select, "snare");
    select.handle_key(press(KeyCode::Char('x')));
    select.handle_key(press(KeyCode::Char('j')));

    select.handle_key(press(KeyCode::Esc));

    assert!(select.auto_reverb_overlay_open());
    assert!(select
        .auto_reverb_overlay()
        .unwrap()
        .effect_list()
        .is_none());
    assert_eq!(row_effect(&select, "snare"), before);
}

#[test]
fn closing_an_unchanged_overlay_neither_saves_nor_replays() {
    let mut select = open_with_auto_reverb(DEXED_SNARE, AutoReverbRules::default(), false);
    select.handle_key(press(KeyCode::Char('e')));
    select.handle_key(press(KeyCode::Char('j')));

    assert_eq!(
        select.handle_key(press(KeyCode::Esc)),
        PatchSelectAction::Continue
    );
    assert!(!select.auto_reverb_overlay_open());
    assert!(!select.captures_all_keys());
}

#[test]
fn shift_e_toggles_auto_reverb_and_asks_to_save_and_replay() {
    let mut select = open_with_auto_reverb(DEXED_SNARE, AutoReverbRules::default(), false);

    let PatchSelectAction::SaveAutoReverb { rules, preview } = select.handle_key(shift('E')) else {
        panic!("E must ask the host to save");
    };
    assert!(!rules.enabled());
    assert_eq!(preview.as_deref(), Some(DEXED_SNARE));
    assert_eq!(select.auto_reverb_stage(DEXED_SNARE), None);

    // Shift が修飾に載らない端末の `E` でも戻る。
    let PatchSelectAction::SaveAutoReverb { rules, .. } =
        select.handle_key(press(KeyCode::Char('E')))
    else {
        panic!("E must ask the host to save");
    };
    assert!(rules.enabled());
    assert!(select.auto_reverb_stage(DEXED_SNARE).is_some());
}

#[test]
fn the_stage_follows_the_rules_and_is_withheld_for_builtin_effects_and_existing_chains() {
    let select = open_with_auto_reverb(DEXED_SNARE, AutoReverbRules::default(), false);
    assert_eq!(
        select.auto_reverb_stage(DEXED_SNARE),
        Some(json!({"Dragonfly Room Reverb preset": "Small Drum Room"}))
    );
    assert_eq!(select.auto_reverb_stage(DEXED_BASS), None);
    assert_eq!(select.auto_reverb_stage(SURGE_PAD), None);

    let with_chain = open_with_auto_reverb(DEXED_SNARE, AutoReverbRules::default(), true);
    assert_eq!(with_chain.auto_reverb_stage(DEXED_SNARE), None);
}
