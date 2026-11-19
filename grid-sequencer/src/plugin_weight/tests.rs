use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::auto_reverb::tests::{patch_load, DEXED_PAD, DEXED_SNARE, SURGE_PAD};
use crate::tests::ready_ctx;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// 7 項目の下に plugin 行が `Dexed`、`Surge XT` の順で並ぶ。
const DEXED_ROW: usize = crate::CycleRandomItem::ALL.len();

fn open_overlay(screen: &mut GridSequencerScreen, ctx: &GridSequencerContext<'_>) {
    screen.handle_key(press(KeyCode::Char('a')), Instant::now(), ctx);
    assert!(screen.cycle_random_open());
}

#[test]
fn the_overlay_lists_one_row_per_catalog_plugin() {
    let patches = patch_load();
    let ctx = ready_ctx(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    open_overlay(&mut screen, &ctx);

    let overlay = screen.cycle_random_overlay().expect("開いている");
    assert_eq!(overlay.plugins(), ["Dexed", "Surge XT"]);
}

#[test]
fn space_on_a_plugin_row_toggles_its_weight_between_normal_and_zero() {
    let patches = patch_load();
    let ctx = ready_ctx(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    open_overlay(&mut screen, &ctx);
    for _ in 0..DEXED_ROW {
        screen.handle_key(press(KeyCode::Down), Instant::now(), &ctx);
    }

    screen.handle_key(press(KeyCode::Char(' ')), Instant::now(), &ctx);
    assert_eq!(screen.plugin_weights().weight("Dexed"), PluginWeight::Zero);
    assert_eq!(
        screen.plugin_weights().weight("Surge XT"),
        PluginWeight::Normal
    );
    // 項目の設定は動かさない。
    assert_eq!(screen.cycle_random(), crate::CycleRandom::ALL);

    screen.handle_key(press(KeyCode::Char(' ')), Instant::now(), &ctx);
    assert_eq!(
        screen.plugin_weights().weight("Dexed"),
        PluginWeight::Normal
    );
}

#[test]
fn the_cursor_stops_at_the_last_plugin_row() {
    let patches = patch_load();
    let ctx = ready_ctx(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    open_overlay(&mut screen, &ctx);
    for _ in 0..20 {
        screen.handle_key(press(KeyCode::Down), Instant::now(), &ctx);
    }

    assert_eq!(screen.cycle_random_cursor(), Some(DEXED_ROW + 1));
}

#[test]
fn a_zero_weight_plugin_is_left_out_of_the_random_candidates() {
    let patches = patch_load();
    let ctx = ready_ctx(&patches);
    // drum 行があると snare の Role が予約されて NOTE 候補から外れるので、1 track にする。
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    let note = crate::GridPatchPurpose::Note;
    assert_eq!(
        screen.patch_candidates_for_purpose(note, &ctx).len(),
        3,
        "既定は全 plugin が normal"
    );

    screen.toggle_plugin_weight("Dexed");

    assert_eq!(screen.patch_candidates_for_purpose(note, &ctx), [SURGE_PAD]);
    for _ in 0..10 {
        screen.apply_random_patches(&ctx, false);
        for instance in screen.state.instances() {
            let patch = instance.patch.as_deref();
            assert_ne!(patch, Some(DEXED_PAD));
            assert_ne!(patch, Some(DEXED_SNARE));
        }
    }
}

#[test]
fn the_weights_survive_the_session_round_trip() {
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.toggle_plugin_weight("Surge XT");
    screen.grid_ready = true;

    let session = screen.session_state().expect("grid は作成済み");
    assert_eq!(
        session.plugin_weights.zero_plugins().collect::<Vec<_>>(),
        ["Surge XT"]
    );
    let restored = GridSequencerScreen::new_with(crate::GridSequencerParts {
        track_count: 4,
        restored_session: Some(session),
        ..crate::GridSequencerParts::default()
    });
    assert_eq!(
        restored.plugin_weights().weight("Surge XT"),
        PluginWeight::Zero
    );
}
