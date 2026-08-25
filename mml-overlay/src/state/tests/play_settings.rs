//! `Ctrl+L` の演奏設定は overlay 全体で共通（音色選択の最中にも開ける）。

use super::*;

use crate::line_play::FilterSettings;
use crate::play_settings::PlaySettings;

fn patches() -> Vec<PatchCatalogEntry> {
    vec![PatchCatalogEntry::from_display(
        "Pads/Pad 1.fxp".to_string(),
    )]
}

fn opened_with_patch_select() -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patch_catalog: PatchCatalogSnapshot::Ready(patches()),
        ..MmlOverlayContext::default()
    });
    overlay.handle_key(ctrl(KeyCode::Char('t')), Instant::now());
    assert!(overlay.patch_select().is_some());
    overlay
}

#[test]
fn ctrl_l_opens_the_play_settings() {
    let mut overlay = opened();

    let action = overlay.handle_key(ctrl(KeyCode::Char('l')), Instant::now());

    assert_eq!(action, MmlOverlayAction::Continue);
    assert!(overlay.play_settings_select().is_some());
}

/// 演奏設定は overlay 全体で共通なので、音色選択の最中にも開ける。
#[test]
fn ctrl_l_opens_the_play_settings_from_the_patch_select_too() {
    let mut overlay = opened_with_patch_select();

    overlay.handle_key(ctrl(KeyCode::Char('l')), Instant::now());

    assert!(overlay.play_settings_select().is_some());
    // 音色選択は開いたまま。閉じてしまうと絞り込みのやり直しになる。
    assert!(overlay.patch_select().is_some());
}

/// 開いている間は音色選択へキーが漏れない（最も手前のモーダル）。
#[test]
fn while_open_the_keys_do_not_reach_the_patch_select() {
    let mut overlay = opened_with_patch_select();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);

    let action = overlay.handle_key(press(KeyCode::Down), now);

    assert_eq!(action, MmlOverlayAction::Continue);
    assert_eq!(overlay.play_settings_select().unwrap().cursor(), 1);
}

/// 開いている間は入力欄にも文字が入らない。
#[test]
fn while_open_the_space_key_does_not_type_into_the_input() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);

    overlay.handle_key(press(KeyCode::Char(' ')), now);

    assert_eq!(overlay.value(), "cde");
    assert!(!overlay.play_settings().repeat);
    assert!(overlay.play_settings_select().unwrap().settings().repeat);
}

#[test]
fn enter_confirms_the_three_values_and_closes() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Down), now);
    overlay.handle_key(press(KeyCode::Down), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);

    overlay.handle_key(press(KeyCode::Enter), now);

    assert!(overlay.play_settings_select().is_none());
    assert_eq!(
        overlay.play_settings(),
        PlaySettings {
            repeat: true,
            filters: FilterSettings {
                modulation: false,
                velocity: true,
            },
        }
    );
}

#[test]
fn esc_rolls_back_to_the_values_it_opened_with() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Enter), now);
    let confirmed = overlay.play_settings();

    overlay.handle_key(ctrl(KeyCode::Char('l')), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Down), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Esc), now);

    assert!(overlay.play_settings_select().is_none());
    assert_eq!(overlay.play_settings(), confirmed);
    assert!(confirmed.repeat);
}

/// 演奏設定を閉じる Esc は overlay 自体を閉じない。
#[test]
fn esc_closes_only_the_play_settings() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);

    let action = overlay.handle_key(press(KeyCode::Esc), now);

    assert_eq!(action, MmlOverlayAction::Continue);
    assert!(overlay.is_open());
}

/// 確定した設定は overlay を開き直しても残る（セッションへ保存する値のため）。
#[test]
fn the_settings_survive_reopening_the_overlay() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Enter), now);
    overlay.handle_key(press(KeyCode::Esc), now);

    overlay.open(MmlOverlayContext::default());

    assert!(overlay.play_settings().repeat);
    assert!(overlay.play_settings_select().is_none());
}

/// 確定した設定はそのまま演奏へ載る。ここが繋がっていないと `Ctrl+L` は飾りになる。
#[test]
fn the_confirmed_settings_ride_on_the_next_line_playback() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Down), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    let action = overlay.handle_key(ctrl(KeyCode::Char(' ')), now);

    let MmlOverlayAction::PlayLine { program, .. } = action else {
        panic!("行を鳴らすはず: {action:?}");
    };
    assert!(program.repeat);
    assert!(program.filters.modulation);
    assert!(!program.filters.velocity);
    assert!(!program.is_silent());
}

/// 既定は今までどおり 1 回だけ。設定を触っていない人の音は変わらない。
#[test]
fn without_touching_the_settings_a_line_still_plays_once() {
    let mut overlay = opened();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);

    let action = overlay.handle_key(ctrl(KeyCode::Char(' ')), now);

    let MmlOverlayAction::PlayLine { program, .. } = action else {
        panic!("行を鳴らすはず: {action:?}");
    };
    assert!(!program.repeat);
    assert_eq!(program.filters, FilterSettings::default());
}

/// 設定は overlay 全体で共通。履歴の試聴も同じ設定で鳴る。
#[test]
fn the_history_preview_uses_the_same_settings() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        history: vec!["cde".to_string(), "gab".to_string()],
        ..MmlOverlayContext::default()
    });
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Enter), now);
    overlay.handle_key(ctrl(KeyCode::Char('o')), now);

    let action = overlay.handle_key(press(KeyCode::Down), now);

    let MmlOverlayAction::PlayLine { program, .. } = action else {
        panic!("履歴を試聴するはず: {action:?}");
    };
    assert!(program.repeat);
}
