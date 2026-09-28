use super::*;

use cmrt_core::EffectPlugins;
use serde_json::json;

use crate::auto_reverb::tests::{
    test_catalog, DEXED_BASS, DEXED_KICK, DEXED_PAD, DEXED_SNARE, SURGE_PAD,
};
use crate::auto_reverb::{AutoReverbRules, HostChain};

/// 控えの無い reverb が 1 段ある（手動 reverb の）chain。
fn manual_reverb_chain() -> HostChain {
    HostChain::from_json(
        Some(&json!({
            cmrt_core::EFFECT_CHAIN_JSON_KEY: [{"Dragonfly Hall Reverb preset": "Medium Clear Hall"}],
        })),
        Some(&test_catalog()),
    )
}

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
    chain: HostChain,
) -> PatchSelect<'static> {
    PatchSelect::open(PatchSelectRequest {
        patches: auto_reverb_patches(),
        current: Some(current.to_string()),
        auto_reverb: Some(AutoReverbHost {
            rules,
            effect_plugins: EffectPlugins::with_catalog(test_catalog()),
            chain,
        }),
        ..Default::default()
    })
    .expect("patch list is not empty")
}

pub(super) fn shift(ch: char) -> KeyEvent {
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

/// ルール overlay に出ている行での位置。
fn shown_index(select: &PatchSelect<'_>, name: &str) -> usize {
    let row = row_index(select, name);
    select
        .auto_reverb_overlay()
        .unwrap()
        .shown_rows()
        .iter()
        .position(|index| *index == row)
        .unwrap()
}

pub(super) fn move_overlay_to(select: &mut PatchSelect<'_>, name: &str) {
    let target = shown_index(select, name);
    while select.auto_reverb_overlay().unwrap().cursor() < target {
        select.handle_key(press(KeyCode::Char('j')));
    }
    assert_eq!(select.auto_reverb_overlay().unwrap().cursor(), target);
}

pub(super) fn effect_list_labels(select: &PatchSelect<'_>) -> Vec<String> {
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

pub(super) fn row_effect(select: &PatchSelect<'_>, name: &str) -> Option<serde_json::Value> {
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

    assert_eq!(select.handle_key(shift('E')), PatchSelectAction::Continue);
    assert_eq!(
        select.handle_key(press(KeyCode::Char('e'))),
        PatchSelectAction::Continue
    );
    assert!(!select.auto_reverb_overlay_open());
    assert_eq!(select.auto_reverb_status(), None);
    assert_eq!(select.auto_reverb_stage(DEXED_SNARE), None);
}

#[test]
fn shift_e_opens_the_rules_overlay_and_it_captures_every_key() {
    let mut select = open_with_auto_reverb(
        DEXED_SNARE,
        AutoReverbRules::default(),
        HostChain::default(),
    );

    select.handle_key(shift('E'));

    assert!(select.auto_reverb_overlay_open());
    assert!(select.captures_all_keys());
    // overlay の j は音色カーソルではなく行を動かす。
    select.handle_key(press(KeyCode::Char('j')));
    assert_eq!(select.selected(), Some(DEXED_SNARE));
    assert_eq!(select.auto_reverb_overlay().unwrap().cursor(), 1);
    select.handle_key(press(KeyCode::Char('k')));
    select.handle_key(press(KeyCode::Up));
    assert_eq!(select.auto_reverb_overlay().unwrap().cursor(), 0);
    // overlay の Enter は音色を確定せず、effect list を開いて試聴する。
    assert_eq!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Preview(DEXED_SNARE.to_string())
    );
    assert!(select
        .auto_reverb_overlay()
        .unwrap()
        .effect_list()
        .is_some());
}

#[test]
fn closing_an_unchanged_overlay_neither_saves_nor_replays() {
    let mut select = open_with_auto_reverb(
        DEXED_SNARE,
        AutoReverbRules::default(),
        HostChain::default(),
    );
    select.handle_key(shift('E'));
    select.handle_key(press(KeyCode::Char('j')));

    assert_eq!(
        select.handle_key(press(KeyCode::Esc)),
        PatchSelectAction::Continue
    );
    assert!(!select.auto_reverb_overlay_open());
    assert!(!select.captures_all_keys());
}

#[test]
fn e_toggles_auto_reverb_and_asks_to_save_and_replay() {
    let mut select = open_with_auto_reverb(
        DEXED_SNARE,
        AutoReverbRules::default(),
        HostChain::default(),
    );

    let PatchSelectAction::SaveAutoReverb { rules, preview } =
        select.handle_key(press(KeyCode::Char('e')))
    else {
        panic!("e must ask the host to save");
    };
    assert!(!rules.enabled());
    assert_eq!(preview.as_deref(), Some(DEXED_SNARE));
    assert_eq!(select.auto_reverb_stage(DEXED_SNARE), None);

    let PatchSelectAction::SaveAutoReverb { rules, .. } =
        select.handle_key(press(KeyCode::Char('e')))
    else {
        panic!("e must ask the host to save");
    };
    assert!(rules.enabled());
    assert!(select.auto_reverb_stage(DEXED_SNARE).is_some());
}

#[test]
fn the_stage_follows_the_rules_and_is_withheld_for_builtin_effects_and_manual_reverbs() {
    let select = open_with_auto_reverb(
        DEXED_SNARE,
        AutoReverbRules::default(),
        HostChain::default(),
    );
    assert_eq!(
        select.auto_reverb_stage(DEXED_SNARE),
        Some(json!({"Dragonfly Room Reverb preset": "Small Drum Room"}))
    );
    assert_eq!(select.auto_reverb_stage(DEXED_BASS), None);
    assert_eq!(select.auto_reverb_stage(SURGE_PAD), None);

    let with_chain = open_with_auto_reverb(
        DEXED_SNARE,
        AutoReverbRules::default(),
        manual_reverb_chain(),
    );
    assert_eq!(with_chain.auto_reverb_stage(DEXED_SNARE), None);
}

#[test]
fn the_overlay_shows_a_user_added_row_only_for_roles_with_user_presets() {
    let shown_names = |user_presets: Vec<(String, String)>| {
        let mut select = PatchSelect::open(PatchSelectRequest {
            patches: auto_reverb_patches(),
            current: Some(DEXED_SNARE.to_string()),
            user_presets,
            auto_reverb: Some(AutoReverbHost {
                rules: AutoReverbRules::default(),
                effect_plugins: EffectPlugins::with_catalog(test_catalog()),
                chain: HostChain::default(),
            }),
            ..Default::default()
        })
        .unwrap();
        select.handle_key(shift('E'));
        let rows = select.auto_reverb_rules().unwrap().rows();
        select
            .auto_reverb_overlay()
            .unwrap()
            .shown_rows()
            .iter()
            .map(|index| rows[*index].0.name())
            .collect::<Vec<_>>()
    };

    let without = shown_names(Vec::new());
    assert!(!without.contains(&"lead ユーザー追加".to_string()));
    assert!(without.contains(&"etc 未分類".to_string()));
    let with = shown_names(vec![("lead".to_string(), "sync".to_string())]);
    assert!(with.contains(&"lead ユーザー追加".to_string()));
    assert!(!with.contains(&"bass ユーザー追加".to_string()));
}
