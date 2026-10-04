use cmrt_core::{AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset, EffectPlugins};
use cmrt_effect_chain_select::{messages, stage_is_bypassed};
use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::patch_load::PatchLoadState;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use serde_json::Value;

use crate::{
    KeyboardContext, KeyboardMmlInput, KeyboardNoteGuide, KeyboardScreen, KeyboardState,
    KeyboardVoicingLookup, PatchPaneFocus, KEYBOARD_NOTES,
};

struct NoVoicing;

impl KeyboardVoicingLookup for NoVoicing {
    fn cached_voicing(&self, _patch: &str) -> Option<PatchVoicing> {
        None
    }
}

pub(crate) fn patch_load() -> PatchLoadState {
    PatchLoadState::ready(
        ["Leads/Lead 1.fxp", "Pads/Warm.fxp"]
            .iter()
            .map(|name| (name.to_string(), name.to_lowercase()))
            .collect(),
    )
}

pub(crate) fn context(patch_load: &PatchLoadState) -> KeyboardContext<'_> {
    KeyboardContext {
        patch_dirs_configured: true,
        patch_load,
        voicing: &NoVoicing,
        catalog_notes: &[],
    }
}

/// 候補は catalog 登録順で `Delay/Echo`・`Clean`。
pub(crate) fn catalog() -> AudioEffectCatalog {
    let fx = AudioEffectPluginInfo::new(
        "Test FX",
        "/clap/does-not-exist.clap",
        "org.example.fx",
        "/presets/does-not-exist",
    );
    let amp = AudioEffectPluginInfo::new(
        "Test Amp",
        "/clap/does-not-exist-amp.clap",
        "org.example.amp",
        "/presets/does-not-exist-amp",
    );
    let preset = |plugin: &AudioEffectPluginInfo, value: &str, category: &str, kind: &str| {
        AudioEffectPreset {
            plugin: plugin.key.clone(),
            json_key: plugin.json_key.clone(),
            value: value.to_string(),
            display: format!("{}: {value}", plugin.name),
            name: value.to_string(),
            category: category.to_string(),
            kind: kind.to_string(),
            path: std::path::PathBuf::from(format!("/presets/{value}")),
        }
    };
    let presets = vec![
        preset(&fx, "Delay/Echo", "Space / Imaging", "Delay"),
        preset(&amp, "Clean", "Distortion / Saturation", "Amp Simulator"),
    ];
    AudioEffectCatalog::with_entries(vec![fx, amp], presets)
}

fn stage(index: usize) -> Value {
    catalog().presets()[index].json_element()
}

pub(crate) fn screen_with(
    effect_plugins: EffectPlugins,
    chain: Vec<Value>,
    ctx: &KeyboardContext<'_>,
) -> KeyboardScreen<'static> {
    let mut screen = KeyboardScreen::new(
        None,
        KeyboardState::new(Some("Leads/Lead 1.fxp".to_string())),
        KeyboardMmlInput::default(),
        KeyboardNoteGuide::new(None),
    )
    .with_effect_plugins(effect_plugins)
    .with_effect_chain(chain);
    screen.sync_patch_catalog(ctx);
    screen
}

/// focus を Effect pane へ移した画面。
fn focused(chain: Vec<Value>, ctx: &KeyboardContext<'_>) -> KeyboardScreen<'static> {
    let mut screen = screen_with(EffectPlugins::with_catalog(catalog()), chain, ctx);
    press(&mut screen, 'l', ctx);
    assert_eq!(screen.state.patch_catalog.focus(), PatchPaneFocus::Effect);
    screen
}

fn key(screen: &mut KeyboardScreen<'_>, key: KeyEvent, ctx: &KeyboardContext<'_>) {
    screen.handle_key(key, ctx);
}

fn press(screen: &mut KeyboardScreen<'_>, ch: char, ctx: &KeyboardContext<'_>) {
    key(
        screen,
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
        ctx,
    );
}

fn code(screen: &mut KeyboardScreen<'_>, code: KeyCode, ctx: &KeyboardContext<'_>) {
    key(screen, KeyEvent::new(code, KeyModifiers::NONE), ctx);
}

/// 押さえた音を 1 つ置いておく。接続前の音符 Press は押さえた音を捨てるので、
/// キーの後に残っていれば、そのキーは音符として扱われていない。
fn hold_a_note(screen: &mut KeyboardScreen<'_>) {
    assert!(screen.state.press(KEYBOARD_NOTES[0]).is_some());
}

fn note_was_played(screen: &KeyboardScreen<'_>) -> bool {
    screen.state.held().is_empty()
}

#[test]
fn l_three_times_from_role_reaches_effect_and_stops_there() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen_with(EffectPlugins::none(), Vec::new(), &ctx);
    press(&mut screen, 'h', &ctx);
    press(&mut screen, 'h', &ctx);
    assert_eq!(screen.state.patch_catalog.focus(), PatchPaneFocus::Role);
    for _ in 0..3 {
        press(&mut screen, 'l', &ctx);
    }
    assert_eq!(screen.state.patch_catalog.focus(), PatchPaneFocus::Effect);
    press(&mut screen, 'l', &ctx);
    assert_eq!(screen.state.patch_catalog.focus(), PatchPaneFocus::Effect);
}

#[test]
fn a_opens_the_add_overlay_without_playing_a4() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(Vec::new(), &ctx);
    hold_a_note(&mut screen);

    press(&mut screen, 'a', &ctx);

    assert!(screen.effect_pane().is_adding());
    assert!(!note_was_played(&screen));
    // 開いた時点で list の先頭の候補を試聴として掛ける。
    assert_eq!(screen.effect_pane().sounding(), [stage(0)]);
    assert!(screen.effect_pane().chain().is_empty());
}

#[test]
fn dd_deletes_but_a_single_d_does_not() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(vec![stage(0), stage(1)], &ctx);
    hold_a_note(&mut screen);

    press(&mut screen, 'd', &ctx);
    assert_eq!(screen.effect_pane().chain().len(), 2);
    assert!(!note_was_played(&screen));
    press(&mut screen, 'd', &ctx);
    assert_eq!(screen.effect_pane().chain(), [stage(1)]);
    // chain を変えたらすぐ鳴っている音へ掛け直す。
    assert_eq!(screen.effect_pane().sounding(), [stage(1)]);
}

#[test]
fn a_d_then_another_key_does_not_delete() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(vec![stage(0), stage(1)], &ctx);

    press(&mut screen, 'd', &ctx);
    press(&mut screen, 'c', &ctx);
    press(&mut screen, 'd', &ctx);
    assert_eq!(screen.effect_pane().chain().len(), 2);
}

#[test]
fn b_toggles_bypass_and_sounds_the_new_chain() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(vec![stage(0)], &ctx);
    hold_a_note(&mut screen);

    press(&mut screen, 'b', &ctx);

    assert!(stage_is_bypassed(&screen.effect_pane().chain()[0]));
    assert_eq!(
        screen.effect_pane().sounding(),
        screen.effect_pane().chain()
    );
    assert!(!note_was_played(&screen));
}

#[test]
fn alt_down_moves_the_stage_and_j_moves_the_cursor() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(vec![stage(0), stage(1)], &ctx);

    key(
        &mut screen,
        KeyEvent::new(KeyCode::Down, KeyModifiers::ALT),
        &ctx,
    );
    assert_eq!(screen.effect_pane().chain(), [stage(1), stage(0)]);
    assert_eq!(screen.effect_pane().sounding(), [stage(1), stage(0)]);
    assert_eq!(screen.effect_pane().editor().cursor, 1);

    press(&mut screen, 'k', &ctx);
    assert_eq!(screen.effect_pane().editor().cursor, 0);
    code(&mut screen, KeyCode::End, &ctx);
    assert_eq!(screen.effect_pane().editor().cursor, 1);
    // j/k は chain を変えないので音色も chain も変わらない。
    assert_eq!(screen.state.patch(), Some("Leads/Lead 1.fxp"));
}

#[test]
fn c_still_plays_while_the_effect_pane_is_focused() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(Vec::new(), &ctx);
    hold_a_note(&mut screen);

    press(&mut screen, 'c', &ctx);

    assert!(note_was_played(&screen));
}

#[test]
fn enter_esc_and_space_do_nothing_in_the_pane() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(vec![stage(0)], &ctx);
    for code_ in [KeyCode::Enter, KeyCode::Esc, KeyCode::Char(' ')] {
        code(&mut screen, code_, &ctx);
    }
    assert_eq!(screen.effect_pane().chain(), [stage(0)]);
    assert_eq!(screen.state.patch_catalog.focus(), PatchPaneFocus::Effect);
}

#[test]
fn moving_the_add_cursor_sounds_the_candidate_and_enter_adds_it() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(vec![stage(0)], &ctx);
    press(&mut screen, 'a', &ctx);

    press(&mut screen, 'j', &ctx);
    assert_eq!(screen.effect_pane().sounding(), [stage(0), stage(1)]);
    assert_eq!(screen.effect_pane().chain(), [stage(0)]);

    code(&mut screen, KeyCode::Enter, &ctx);
    assert!(!screen.effect_pane().is_adding());
    assert_eq!(screen.effect_pane().chain(), [stage(0), stage(1)]);
    assert_eq!(screen.effect_pane().sounding(), [stage(0), stage(1)]);
}

#[test]
fn esc_in_the_add_overlay_goes_back_to_the_chain_before_opening() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(vec![stage(0)], &ctx);
    press(&mut screen, 'a', &ctx);
    press(&mut screen, 'j', &ctx);

    code(&mut screen, KeyCode::Esc, &ctx);

    assert!(!screen.effect_pane().is_adding());
    assert_eq!(screen.effect_pane().chain(), [stage(0)]);
    assert_eq!(screen.effect_pane().sounding(), [stage(0)]);
}

#[test]
fn r_replaces_the_cursor_stage() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(vec![stage(0)], &ctx);
    press(&mut screen, 'r', &ctx);
    assert!(screen.effect_pane().is_adding());

    press(&mut screen, 'j', &ctx);
    assert_eq!(screen.effect_pane().sounding(), [stage(1)]);
    code(&mut screen, KeyCode::Enter, &ctx);
    assert_eq!(screen.effect_pane().chain(), [stage(1)]);
}

#[test]
fn the_add_overlay_lets_notes_through_except_b() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(Vec::new(), &ctx);
    press(&mut screen, 'a', &ctx);

    hold_a_note(&mut screen);
    press(&mut screen, 'b', &ctx);
    assert!(!note_was_played(&screen));
    let sounding = screen.effect_pane().sounding();
    assert_eq!(sounding.len(), 1);
    assert!(stage_is_bypassed(&sounding[0]));

    for note in ['c', 'd', 'e', 'f', 'g', 'a'] {
        if screen.state.held().is_empty() {
            hold_a_note(&mut screen);
        }
        press(&mut screen, note, &ctx);
        assert!(note_was_played(&screen), "{note}");
        assert!(screen.effect_pane().is_adding(), "{note}");
    }
}

#[test]
fn a_release_in_the_add_overlay_is_note_off() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(Vec::new(), &ctx);
    press(&mut screen, 'a', &ctx);
    hold_a_note(&mut screen);

    key(
        &mut screen,
        KeyEvent::new_with_kind(
            KeyCode::Char('c'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ),
        &ctx,
    );

    assert!(screen.state.held().is_empty());
    assert!(screen.effect_pane().is_adding());
}

#[test]
fn without_a_catalog_the_pane_says_so_and_a_does_nothing() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen_with(EffectPlugins::none(), Vec::new(), &ctx);
    press(&mut screen, 'l', &ctx);
    hold_a_note(&mut screen);

    press(&mut screen, 'a', &ctx);

    assert!(!screen.effect_pane().is_adding());
    assert!(!note_was_played(&screen));
    assert_eq!(
        screen
            .effect_pane()
            .empty_notice(screen.effect_plugins().is_available()),
        messages::NOT_AVAILABLE_ON_THIS_BACKEND
    );
}

#[test]
fn without_presets_a_says_so_and_does_not_open() {
    let load = patch_load();
    let ctx = context(&load);
    let empty = EffectPlugins::with_catalog(AudioEffectCatalog::with_entries(vec![], vec![]));
    let mut screen = screen_with(empty, Vec::new(), &ctx);
    press(&mut screen, 'l', &ctx);
    assert_eq!(
        screen.effect_pane().empty_notice(true),
        messages::EMPTY_CHAIN
    );

    press(&mut screen, 'a', &ctx);

    assert!(!screen.effect_pane().is_adding());
    assert_eq!(
        screen.effect_pane().empty_notice(true),
        messages::NO_PRESETS
    );
}

#[test]
fn the_chain_is_saved_and_survives_start_and_patch_changes() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(vec![stage(0), stage(1)], &ctx);
    assert_eq!(screen.session_state().effect_chain, [stage(0), stage(1)]);

    // 音色を替えても画面側は chain を送り直さない（sender が覚えた chain を載せる）。
    press(&mut screen, 'h', &ctx);
    press(&mut screen, 'j', &ctx);
    assert_eq!(screen.state.patch(), Some("Pads/Warm.fxp"));
    assert_eq!(screen.effect_pane().sounding(), [stage(0), stage(1)]);

    screen.start(Some("Leads/Lead 1.fxp".to_string()), &ctx);
    assert_eq!(screen.effect_pane().chain(), [stage(0), stage(1)]);
    assert_eq!(screen.session_state().effect_chain, [stage(0), stage(1)]);
}

#[test]
fn reentering_the_screen_closes_the_add_overlay_and_drops_the_audition() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = focused(vec![stage(0)], &ctx);
    press(&mut screen, 'a', &ctx);
    press(&mut screen, 'j', &ctx);
    assert!(screen.blocks_screen_switch());

    screen.resume(&ctx);

    assert!(!screen.effect_pane().is_adding());
    assert!(!screen.blocks_screen_switch());
    assert_eq!(screen.effect_pane().chain(), [stage(0)]);
    assert_eq!(screen.effect_pane().sounding(), [stage(0)]);
}
