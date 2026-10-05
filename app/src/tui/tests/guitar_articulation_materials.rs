//! Guitar Articulation 画面のアルペジエーター overlay で足した素材リストが、設定 file へ書かれ、
//! 次の起動で読まれるか。overlay で変えた行全体のルールが閉じたとき履歴 file へ書かれるか。保存先は `set_local_dir_envs` で temp へ隔離する。

use super::*;
use crate::screen_switch::PrimaryScreen;

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn a_material_added_in_the_overlay_is_saved_and_restored_on_the_next_start() {
    let tmp = crate::test_utils::unique_test_dir("ga_arp_materials");
    std::fs::remove_dir_all(&tmp).ok();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    for code in [KeyCode::Esc, KeyCode::Char('z'), KeyCode::Tab] {
        app.handle_guitar_articulation_key_event(plain(code));
    }
    for ch in "Am7".chars() {
        app.handle_guitar_articulation_key_event(plain(KeyCode::Char(ch)));
    }
    app.handle_guitar_articulation_key_event(plain(KeyCode::Tab));
    assert_eq!(app.guitar_articulation.arp_materials(), ["Am7"]);
    assert_eq!(app.guitar_articulation.arp_material(), "Am7");
    assert_eq!(
        crate::tui::guitar_articulation::load_settings().arp_materials,
        ["Am7"]
    );
    drop(app);

    let cfg = test_config();
    let restarted = TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::none()).unwrap();
    assert_eq!(restarted.guitar_articulation.arp_materials(), ["Am7"]);

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn the_overlay_material_and_arp_are_saved_and_the_next_start_opens_with_them() {
    let tmp = crate::test_utils::unique_test_dir("ga_arp_overlay_state");
    std::fs::remove_dir_all(&tmp).ok();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    for code in [KeyCode::Esc, KeyCode::Char('z'), KeyCode::Tab] {
        app.handle_guitar_articulation_key_event(plain(code));
    }
    for ch in "Am7".chars() {
        app.handle_guitar_articulation_key_event(plain(KeyCode::Char(ch)));
    }
    // 素材 pane を閉じてカーソル行の Am7 を選び、音型の行で 1 つ進めて overlay を閉じる。
    for code in [
        KeyCode::Tab,
        KeyCode::Char('j'),
        KeyCode::Char('l'),
        KeyCode::Esc,
    ] {
        app.handle_guitar_articulation_key_event(plain(code));
    }
    let chosen = *app.guitar_articulation.arp();
    assert_ne!(
        chosen,
        crate::tui::guitar_articulation::ArpSettings::default()
    );
    let saved = crate::tui::guitar_articulation::load_settings();
    assert_eq!(saved.arp_material, "Am7");
    assert_eq!(saved.arp, chosen);
    drop(app);

    let cfg = test_config();
    let mut restarted = TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::none()).unwrap();
    restarted.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    for code in [KeyCode::Esc, KeyCode::Char('z')] {
        restarted.handle_guitar_articulation_key_event(plain(code));
    }
    assert!(restarted.guitar_articulation.arp_overlay_open());
    assert_eq!(restarted.guitar_articulation.arp_material(), "Am7");
    assert_eq!(restarted.guitar_articulation.sounding_mml(), "Am7");
    assert_eq!(restarted.guitar_articulation.applied_arp(), Some(&chosen));

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn a_row_rule_changed_in_the_overlay_is_saved_on_close_and_restored_on_the_next_start() {
    let tmp = crate::test_utils::unique_test_dir("ga_arp_row_rules");
    std::fs::remove_dir_all(&tmp).ok();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::GuitarArticulation, None);
    for code in [KeyCode::Esc, KeyCode::Char('z')] {
        app.handle_guitar_articulation_key_event(plain(code));
    }
    // 最後の行（アクセント）の 1 つ前の奏法でエコへ進めて閉じる。
    for _ in 0..app.guitar_articulation.arp_rows().len() {
        app.handle_guitar_articulation_key_event(plain(KeyCode::Char('j')));
    }
    for code in [KeyCode::Char('k'), KeyCode::Char('l'), KeyCode::Esc] {
        app.handle_guitar_articulation_key_event(plain(code));
    }
    let rules = app.guitar_articulation.rules().clone();
    assert!(rules.is_row_on(crate::tui::guitar_articulation::RowRule::EconomyPicking));
    let saved = crate::tui::guitar_articulation::load_history();
    assert_eq!(saved.entries[0].rules, rules);
    drop(app);

    let cfg = test_config();
    let restarted = TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::none()).unwrap();
    assert_eq!(*restarted.guitar_articulation.rules(), rules);

    std::fs::remove_dir_all(&tmp).ok();
}
