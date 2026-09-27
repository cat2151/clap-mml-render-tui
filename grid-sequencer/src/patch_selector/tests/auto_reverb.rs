use cmrt_patch_select::auto_reverb::AutoReverbRules;

use super::*;
use crate::auto_reverb::tests::{
    effect_plugins, patch_load, DEXED_SNARE, DRUM_ROOM_CHAIN, SURGE_PAD,
};
use crate::{GridSequencerAction, GridSequencerParts, NoVoicingLookup};

/// 2 行。0 行目は Surge の pad、1 行目は Dexed の snare。patch catalog は [`patch_load`]。
fn screen_with_auto_reverb() -> GridSequencerScreen {
    let mut screen = GridSequencerScreen::new_with(GridSequencerParts {
        track_count: 2,
        effect_plugins: effect_plugins(),
        ..GridSequencerParts::default()
    });
    screen.auto_reverb.observe(&patch_load());
    screen.state.rows_mut()[0].patch = Some(SURGE_PAD.to_string());
    screen.state.rows_mut()[1].patch = Some(DEXED_SNARE.to_string());
    screen
}

fn ctx(patch_load: &PatchLoadState) -> GridSequencerContext<'_> {
    ctx_with(patch_load, crate::tests::empty_catalog(), &NoVoicingLookup)
}

/// 準備で送った chain。`reason` と行で引く。
fn sent_chains(screen: &GridSequencerScreen, reason: &str, row: usize) -> Vec<String> {
    let instance_id = screen.state.instance_id(row);
    screen
        .sent_patches
        .borrow()
        .iter()
        .filter(|sent| sent.reason == reason && sent.instance_id == instance_id)
        .map(|sent| sent.patch.effect_chain.clone())
        .collect()
}

/// selector で `patch` へカーソルを置き、試聴させる。
fn preview(screen: &mut GridSequencerScreen, patch: &str, ctx: &GridSequencerContext<'_>) {
    let selector = screen.patch_selector.as_mut().expect("selector is open");
    // Role ALL の Preset ALL は全音色。
    selector.select_role(0);
    let index = (0..selector.filtered_len())
        .position(|index| selector.filtered_entry(index).map(|e| e.display()) == Some(patch))
        .expect("patch is listed under ALL");
    selector.select_patch(index);
    screen.preview_patch_selection(ctx);
}

#[test]
fn preview_confirm_and_playback_send_the_same_drum_room_chain() {
    let patch_load = patch_load();
    let ctx = ctx(&patch_load);
    let mut screen = screen_with_auto_reverb();

    screen.open_patch_selector(0, &ctx);
    preview(&mut screen, DEXED_SNARE, &ctx);
    screen.handle_key(press(KeyCode::Enter), Instant::now(), &ctx);
    screen.prepare_connection();
    let row0_id = screen.state.instance_id(0);
    screen.send_preload(row0_id, Some(DEXED_SNARE));

    assert_eq!(sent_chains(&screen, "preview", 0), [DRUM_ROOM_CHAIN]);
    assert_eq!(sent_chains(&screen, "confirm", 0), [DRUM_ROOM_CHAIN]);
    assert_eq!(sent_chains(&screen, "prepare", 0), [DRUM_ROOM_CHAIN]);
    assert_eq!(sent_chains(&screen, "preload", 0), [DRUM_ROOM_CHAIN]);
}

#[test]
fn a_surge_row_is_prepared_without_a_chain() {
    let mut screen = screen_with_auto_reverb();

    screen.prepare_connection();

    assert_eq!(sent_chains(&screen, "prepare", 0), [""]);
    assert_eq!(sent_chains(&screen, "prepare", 1), [DRUM_ROOM_CHAIN]);
}

#[test]
fn shift_e_turns_auto_reverb_off_and_reprepares_only_rows_whose_chain_changes() {
    let patch_load = patch_load();
    let ctx = ctx(&patch_load);
    let mut screen = screen_with_auto_reverb();
    screen.open_patch_selector(0, &ctx);

    let action = screen.handle_key(
        KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT),
        Instant::now(),
        &ctx,
    );

    let GridSequencerAction::SaveAutoReverb(rules) = action else {
        panic!("E asks the app to save the rules");
    };
    assert!(!rules.enabled());
    // Surge の行は chain が変わらないので送り直さない。snare の行は chain を外す。
    assert_eq!(sent_chains(&screen, "auto-reverb", 0), Vec::<String>::new());
    assert_eq!(sent_chains(&screen, "auto-reverb", 1), [""]);
    screen.prepare_connection();
    assert_eq!(sent_chains(&screen, "prepare", 1), [""]);
}

#[test]
fn the_selector_row_is_compared_with_the_patch_being_previewed() {
    let patch_load = patch_load();
    let ctx = ctx(&patch_load);
    let mut screen = screen_with_auto_reverb();
    screen.open_patch_selector(0, &ctx);
    preview(&mut screen, DEXED_SNARE, &ctx);

    screen.handle_key(
        KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT),
        Instant::now(),
        &ctx,
    );

    // 0 行目の確定値は Surge のままだが、鳴っているのは試聴中の snare。
    assert_eq!(sent_chains(&screen, "auto-reverb", 0), [""]);
}

#[test]
fn e_opens_the_rules_overlay_which_takes_every_key_until_esc() {
    let patch_load = patch_load();
    let ctx = ctx(&patch_load);
    let mut screen = screen_with_auto_reverb();
    screen.open_patch_selector(0, &ctx);

    screen.handle_key(press(KeyCode::Char('e')), Instant::now(), &ctx);
    assert!(selector(&screen).auto_reverb.overlay_open());
    // selector の `q`（取り消して閉じる）には届かない。
    screen.handle_key(press(KeyCode::Char('q')), Instant::now(), &ctx);
    assert!(selector(&screen).auto_reverb.overlay_open());

    let action = screen.handle_key(press(KeyCode::Esc), Instant::now(), &ctx);
    assert!(matches!(action, GridSequencerAction::Continue));
    assert!(!selector(&screen).auto_reverb.overlay_open());
    assert!(screen.patch_selector.is_some());
}

#[test]
fn rules_set_by_the_app_apply_to_the_next_prepare() {
    let mut screen = screen_with_auto_reverb();
    let mut rules = AutoReverbRules::default();
    rules.set_enabled(false);

    screen.set_auto_reverb_rules(rules);
    screen.prepare_connection();

    assert_eq!(sent_chains(&screen, "prepare", 1), [""]);
}
