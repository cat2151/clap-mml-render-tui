use std::collections::BTreeMap;

use cmrt_patch_select::{HostPatchCatalog, PatchCatalogEntry, PatchCatalogSnapshot};
use cmrt_patches::PatchRoleIndex;
use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::patch_load::PatchLoadState;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    KeyboardContext, KeyboardMmlInput, KeyboardNoteGuide, KeyboardScreen, KeyboardState,
    KeyboardVoicingLookup, KEYBOARD_NOTES,
};

struct NoVoicing;

impl KeyboardVoicingLookup for NoVoicing {
    fn cached_voicing(&self, _patch: &str) -> Option<PatchVoicing> {
        None
    }
}

const PATCHES: [(&str, &str); 4] = [
    ("Bell.syx/00", "Dexed"),
    ("Harp.floe-preset", "Floe"),
    ("Pads/Warm.fxp", "Surge XT"),
    ("Warm Pad.vvp", "Vaporizer2"),
];

fn patch_load() -> PatchLoadState {
    PatchLoadState::ready(
        PATCHES
            .iter()
            .map(|(name, _)| (name.to_string(), name.to_lowercase()))
            .collect(),
    )
}

fn context(patch_load: &PatchLoadState) -> KeyboardContext<'_> {
    KeyboardContext {
        patch_dirs_configured: true,
        patch_load,
        voicing: &NoVoicing,
        catalog_notes: &[],
    }
}

/// plugin 名つきの一覧を読んだ画面。`patch_load()` の一覧は plugin 名を持たないので、
/// catalog だけ plugin 名つきで読み直す（Role の索引は同じ空なので、以後の同期で読み直されない）。
fn screen(ctx: &KeyboardContext<'_>) -> KeyboardScreen<'static> {
    screen_with_state(
        ctx,
        KeyboardState::new(Some("Harp.floe-preset".to_string())),
    )
}

fn screen_with_state(ctx: &KeyboardContext<'_>, state: KeyboardState) -> KeyboardScreen<'static> {
    let patch = state.patch().map(str::to_string);
    let mut screen = KeyboardScreen::new(
        None,
        state,
        KeyboardMmlInput::default(),
        KeyboardNoteGuide::new(None),
    );
    screen.sync_patch_catalog(ctx);
    let entries = PATCHES
        .iter()
        .map(|(name, plugin)| {
            PatchCatalogEntry::new(
                name.to_string(),
                name.to_lowercase(),
                plugin.to_string(),
                None,
            )
        })
        .collect();
    screen.state.patch_catalog.load(
        HostPatchCatalog {
            catalog: PatchCatalogSnapshot::Ready(entries),
            patch_role_index: PatchRoleIndex::default(),
            load_measurements: BTreeMap::new(),
        },
        &[],
        patch.as_deref(),
    );
    screen.sync_patch_catalog(ctx);
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

fn shift(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

/// menu を開いて 1 キー選ぶ。
fn choose(screen: &mut KeyboardScreen<'_>, choice: KeyEvent, ctx: &KeyboardContext<'_>) {
    press(screen, 'M', ctx);
    key(screen, choice, ctx);
}

fn listed(screen: &KeyboardScreen<'_>) -> Vec<String> {
    screen
        .state
        .patch_catalog
        .patches()
        .map(|patch| patch.display().to_string())
        .collect()
}

#[test]
fn capital_m_opens_the_menu_with_or_without_the_shift_modifier() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen(&ctx);
    assert_eq!(listed(&screen).len(), 4);

    press(&mut screen, 'M', &ctx);
    let keys: Vec<(char, String)> = screen
        .plugin_menu()
        .expect("menu is open")
        .items()
        .iter()
        .map(|item| (item.key, item.slug.clone()))
        .collect();
    assert_eq!(
        keys,
        [
            ('d', "dexed".to_string()),
            ('f', "floe".to_string()),
            ('s', "surgext".to_string()),
            ('v', "vaporizer2".to_string()),
        ]
    );
    key(
        &mut screen,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        &ctx,
    );
    assert!(screen.plugin_menu().is_none());

    key(&mut screen, shift('M'), &ctx);
    assert!(screen.plugin_menu().is_some());
    key(
        &mut screen,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        &ctx,
    );

    key(&mut screen, shift('m'), &ctx);
    assert!(screen.plugin_menu().is_some());
}

#[test]
fn plain_m_is_modulation_and_does_not_open_the_menu() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen(&ctx);

    press(&mut screen, 'm', &ctx);

    assert!(screen.plugin_menu().is_none());
}

#[test]
fn lowercase_solos_and_the_condition_narrows_the_patches_pane() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen(&ctx);

    choose(
        &mut screen,
        KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE),
        &ctx,
    );

    assert!(screen.plugin_menu().is_none());
    assert_eq!(screen.state.patch_catalog.filter(), "plugin:vaporizer2");
    assert_eq!(listed(&screen), ["Warm Pad.vvp"]);
    // 今の音色（Floe）が消えたので、残った先頭の音色を鳴らせる状態にする。
    assert_eq!(screen.state.patch(), Some("Warm Pad.vvp"));

    // `/` の欄は menu が書いた条件に空白を1つ足して始まる。
    press(&mut screen, '/', &ctx);
    assert_eq!(screen.patch_filter.value(), "plugin:vaporizer2 ");
}

#[test]
fn uppercase_mutes_and_the_same_key_again_removes_it() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen(&ctx);

    choose(&mut screen, shift('D'), &ctx);
    choose(
        &mut screen,
        KeyEvent::new(KeyCode::Char('S'), KeyModifiers::NONE),
        &ctx,
    );
    assert_eq!(
        screen.state.patch_catalog.filter(),
        "-plugin:dexed -plugin:surgext"
    );
    assert_eq!(listed(&screen), ["Harp.floe-preset", "Warm Pad.vvp"]);

    choose(&mut screen, shift('D'), &ctx);
    assert_eq!(screen.state.patch_catalog.filter(), "-plugin:surgext");
    assert_eq!(listed(&screen).len(), 3);

    choose(
        &mut screen,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
        &ctx,
    );
    choose(
        &mut screen,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
        &ctx,
    );
    assert_eq!(screen.state.patch_catalog.filter(), "");
    assert_eq!(listed(&screen).len(), 4);
}

#[test]
fn regex_terms_from_slash_are_kept() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen(&ctx);
    press(&mut screen, '/', &ctx);
    for ch in "warm".chars() {
        press(&mut screen, ch, &ctx);
    }
    key(
        &mut screen,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        &ctx,
    );

    choose(&mut screen, shift('S'), &ctx);

    assert_eq!(screen.state.patch_catalog.filter(), "warm -plugin:surgext");
    assert_eq!(listed(&screen), ["Warm Pad.vvp"]);
}

#[test]
fn while_open_a_note_press_is_swallowed_and_a_release_is_note_off() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen(&ctx);
    assert!(screen.state.press(KEYBOARD_NOTES[0]).is_some());
    press(&mut screen, 'M', &ctx);

    // menu に無いキー。閉じずに無視し、音符としても扱わない（接続前の音符 Press は
    // 押さえている音を捨てるので、残っていれば音符へ流れていない）。
    press(&mut screen, 'c', &ctx);
    assert!(screen.plugin_menu().is_some());
    assert_eq!(screen.state.held().len(), 1);
    assert_eq!(screen.state.patch_catalog.filter(), "");

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
    assert!(screen.plugin_menu().is_some());
}

#[test]
fn key_repeat_does_not_toggle_twice() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen(&ctx);
    press(&mut screen, 'M', &ctx);

    key(
        &mut screen,
        KeyEvent::new_with_kind(KeyCode::Char('f'), KeyModifiers::NONE, KeyEventKind::Repeat),
        &ctx,
    );

    assert!(screen.plugin_menu().is_some());
    assert_eq!(screen.state.patch_catalog.filter(), "");
}

#[test]
fn reentering_the_screen_closes_the_menu() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen(&ctx);
    press(&mut screen, 'M', &ctx);

    screen.resume(&ctx);

    assert!(screen.plugin_menu().is_none());
}

#[test]
fn solo_and_mute_survive_saving_and_restoring_the_session() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen(&ctx);
    choose(&mut screen, shift('D'), &ctx);
    choose(
        &mut screen,
        KeyEvent::new(KeyCode::Char('S'), KeyModifiers::NONE),
        &ctx,
    );
    let condition = screen.state.patch_catalog.filter().to_string();
    let before = listed(&screen);

    let saved = screen.session_state();
    assert_eq!(saved.patch_filter, condition);
    let restored = screen_with_state(&ctx, KeyboardState::from_session(saved));

    assert_eq!(restored.state.patch_catalog.filter(), condition);
    assert_eq!(listed(&restored), before);
}
