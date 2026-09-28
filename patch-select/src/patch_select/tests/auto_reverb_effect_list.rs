//! ルール overlay から開く effect list の操作と試聴。

use super::auto_reverb::{
    effect_list_labels, move_overlay_to, open_with_auto_reverb, row_effect, shift,
};
use super::*;

use serde_json::json;

use crate::auto_reverb::tests::{DEXED_BASS, DEXED_PAD, DEXED_SNARE};
use crate::auto_reverb::{AutoReverbRules, HostChain};

#[test]
fn x_lists_dry_first_then_selectable_reverbs_only() {
    let mut select = open_with_auto_reverb(
        DEXED_SNARE,
        AutoReverbRules::default(),
        HostChain::default(),
    );
    select.handle_key(shift('E'));
    move_overlay_to(&mut select, "snare");

    select.handle_key(press(KeyCode::Char('x')));

    assert_eq!(
        effect_list_labels(&select),
        [
            "なし(dry)",
            "Dragonfly Room Reverb: Small Drum Room",
            "Dragonfly Room Reverb: Large Drum Room",
            "Dragonfly Room Reverb: Medium Clear Room",
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
fn the_effect_list_moves_by_page_and_jumps_with_home_and_end() {
    let mut select = open_with_auto_reverb(
        DEXED_SNARE,
        AutoReverbRules::default(),
        HostChain::default(),
    );
    select.handle_key(shift('E'));
    move_overlay_to(&mut select, "snare");
    select.handle_key(press(KeyCode::Char('x')));
    let cursor = |select: &PatchSelect<'_>| {
        select
            .auto_reverb_overlay()
            .unwrap()
            .effect_list()
            .unwrap()
            .cursor()
    };
    let last = effect_list_labels(&select).len() - 1;

    select.handle_key(press(KeyCode::End));
    assert_eq!(cursor(&select), last);
    select.handle_key(press(KeyCode::Home));
    assert_eq!(cursor(&select), 0);
    // 候補が PAGE_STEP 件より少ないので、端で止まる。
    select.handle_key(press(KeyCode::PageDown));
    assert_eq!(cursor(&select), last);
    select.handle_key(press(KeyCode::PageUp));
    assert_eq!(cursor(&select), 0);
}

#[test]
fn enter_in_the_effect_list_sets_the_row_and_esc_saves_and_replays() {
    let mut select = open_with_auto_reverb(
        DEXED_SNARE,
        AutoReverbRules::default(),
        HostChain::default(),
    );
    select.handle_key(shift('E'));
    move_overlay_to(&mut select, "snare");
    select.handle_key(press(KeyCode::Char('x')));
    select.handle_key(press(KeyCode::Char('j')));

    // 閉じたら、決めたルールで鳴らし直す。
    assert_eq!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Preview(DEXED_SNARE.to_string())
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
    let mut select =
        open_with_auto_reverb(DEXED_PAD, AutoReverbRules::default(), HostChain::default());
    select.handle_key(shift('E'));
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
    let mut select = open_with_auto_reverb(
        DEXED_SNARE,
        AutoReverbRules::default(),
        HostChain::default(),
    );
    select.handle_key(shift('E'));
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
fn enter_space_h_and_l_open_the_effect_list_like_x() {
    for key in [
        KeyCode::Enter,
        KeyCode::Char(' '),
        KeyCode::Char('h'),
        KeyCode::Char('l'),
        KeyCode::Char('x'),
    ] {
        let mut select = open_with_auto_reverb(
            DEXED_SNARE,
            AutoReverbRules::default(),
            HostChain::default(),
        );
        select.handle_key(shift('E'));
        select.handle_key(press(key));
        assert!(
            select
                .auto_reverb_overlay()
                .unwrap()
                .effect_list()
                .is_some(),
            "{key:?}"
        );
    }
}

#[test]
fn the_effect_list_auditions_the_candidate_on_the_cursor_patch_whatever_the_row() {
    let mut select = open_with_auto_reverb(
        DEXED_SNARE,
        AutoReverbRules::default(),
        HostChain::default(),
    );
    select.handle_key(shift('E'));
    // snare の音色のまま、別の行（bass|bs は dry）の effect を選ぶ。
    move_overlay_to(&mut select, "bass|bs");
    assert_eq!(
        select.handle_key(press(KeyCode::Char('x'))),
        PatchSelectAction::Preview(DEXED_SNARE.to_string())
    );
    assert_eq!(select.auto_reverb_stage(DEXED_SNARE), None);

    assert_eq!(
        select.handle_key(press(KeyCode::Char('j'))),
        PatchSelectAction::Preview(DEXED_SNARE.to_string())
    );
    let small = json!({"Dragonfly Room Reverb preset": "Small Drum Room"});
    assert_eq!(select.auto_reverb_stage(DEXED_SNARE), Some(small.clone()));
    // カーソルに無い音色はルールのまま。
    assert_eq!(select.auto_reverb_stage(DEXED_BASS), None);
    // 端で動かないなら鳴らし直さない。
    select.handle_key(press(KeyCode::Home));
    assert_eq!(
        select.handle_key(press(KeyCode::Char('k'))),
        PatchSelectAction::Continue
    );

    // 閉じたらルールの音へ戻す。
    assert_eq!(
        select.handle_key(press(KeyCode::Esc)),
        PatchSelectAction::Preview(DEXED_SNARE.to_string())
    );
    assert_eq!(select.auto_reverb_stage(DEXED_SNARE), Some(small));
}
