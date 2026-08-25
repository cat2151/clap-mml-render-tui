//! 音色は入力欄と別に持つ（`Ctrl+T` だけが書き換える）。

use super::*;

fn patches() -> Vec<PatchCatalogEntry> {
    ["Leads/Lead 1.fxp", "Pads/Pad 1.fxp"]
        .into_iter()
        .map(|patch| PatchCatalogEntry::from_display(patch.to_string()))
        .collect()
}

/// `Ctrl+L` で repeat だけを ON にして確定する。
fn turn_on_repeat(overlay: &mut MmlOverlay<'_>, now: Instant) {
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Enter), now);
    assert!(overlay.play_settings().repeat);
}

fn played(action: MmlOverlayAction) -> (PatchChange, crate::line_play::LineProgram) {
    let MmlOverlayAction::PlayLine { patch, program } = action else {
        panic!("行を鳴らすはず: {action:?}");
    };
    (patch, program)
}

fn opened_with_patches() -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patch_catalog: PatchCatalogSnapshot::Ready(patches()),
        ..MmlOverlayContext::default()
    });
    overlay
}

#[test]
fn ctrl_t_opens_the_patch_select() {
    let mut overlay = opened_with_patches();

    overlay.handle_key(ctrl(KeyCode::Char('t')), Instant::now());

    assert!(overlay.patch_select().is_some());
}

#[test]
fn adding_a_patch_filter_preset_is_forwarded_to_the_host_for_json_persistence() {
    let patch = "Instruments/Violin.fxp";
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patch_catalog: PatchCatalogSnapshot::Ready(vec![PatchCatalogEntry::from_display(
            patch.to_string(),
        )]),
        ..MmlOverlayContext::default()
    });
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Left), now);
    overlay.handle_key(press(KeyCode::Left), now);
    for _ in 0..3 {
        overlay.handle_key(press(KeyCode::Down), now);
    }
    for ch in "violin".chars() {
        overlay.handle_key(press(KeyCode::Char(ch)), now);
    }

    assert_eq!(
        overlay.handle_key(ctrl(KeyCode::Char('a')), now),
        MmlOverlayAction::SavePatchFilterPresets {
            presets: vec![("lead".to_string(), "violin".to_string())],
            preview: Some((
                patch.to_string(),
                Some(NoteRequest {
                    messages: vec![[0x90, 60, 127]],
                    duration: Duration::from_millis(250),
                })
            )),
        }
    );
}

#[test]
fn ctrl_t_waits_for_the_patch_list_and_opens_when_ready() {
    let mut overlay = opened();

    overlay.handle_key(ctrl(KeyCode::Char('t')), Instant::now());

    assert!(!overlay.is_patch_select_open());
    assert!(overlay.is_waiting_for_patch_catalog());

    let measurements = std::collections::BTreeMap::from([(
        "Leads/Lead 1.fxp".to_string(),
        cmrt_tui_core::patch_load::PatchLoadMeasurement {
            second_load_ms: Some(321),
            ..Default::default()
        },
    )]);
    overlay.sync_patch_catalog(
        PatchCatalogSnapshot::Ready(patches()),
        Default::default(),
        measurements,
    );

    assert!(overlay.is_patch_select_open());
    assert!(!overlay.is_waiting_for_patch_catalog());
    assert_eq!(
        overlay
            .patch_select()
            .unwrap()
            .load_measurement("Leads/Lead 1.fxp")
            .unwrap()
            .second_load_ms,
        Some(321)
    );
}

/// repeat OFF のときの↑↓は今までどおり 1 音の試聴。
#[test]
fn moving_in_the_patch_select_previews_the_patch_with_the_note_at_the_cursor() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Down), now),
        MmlOverlayAction::SetPatch {
            patch: Some("Pads/Pad 1.fxp".to_string()),
            notes: Some(NoteRequest {
                messages: vec![[0x90, 64, 127]],
                duration: Duration::from_millis(250),
            }),
        }
    );
}

#[test]
fn previewing_an_empty_mml_sounds_the_fallback_note() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Down), now),
        MmlOverlayAction::SetPatch {
            patch: Some("Pads/Pad 1.fxp".to_string()),
            // 試聴用の `c` を既定のオクターブ・velocity で鳴らす。
            notes: Some(NoteRequest {
                messages: vec![[0x90, 60, 127]],
                duration: Duration::from_millis(250),
            }),
        }
    );
}

/// 音色は入力欄には現れない。フレーズを 1 行ずつ書き並べる邪魔をしないため。
#[test]
fn confirming_keeps_the_input_untouched() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    assert_eq!(overlay.value(), "cde");
    assert_eq!(overlay.patch(), Some("Leads/Lead 1.fxp"));
    assert!(overlay.patch_select().is_none());
}

#[test]
fn cancelling_restores_the_patch_that_was_current_when_it_opened() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Enter), now);

    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Down), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::SetPatch {
            patch: Some("Leads/Lead 1.fxp".to_string()),
            notes: None,
        }
    );
    assert_eq!(overlay.patch(), Some("Leads/Lead 1.fxp"));
    assert!(overlay.is_open());
}

#[test]
fn cancelling_without_previewing_asks_for_nothing() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::Continue
    );
    assert!(overlay.patch_select().is_none());
}

#[test]
fn reopening_restores_the_patch_but_not_the_mml() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Enter), now);
    overlay.handle_key(press(KeyCode::Esc), now);

    overlay.open(MmlOverlayContext {
        patch_catalog: PatchCatalogSnapshot::Ready(patches()),
        ..MmlOverlayContext::default()
    });

    assert_eq!(overlay.value(), "");
    assert_eq!(overlay.patch(), Some("Leads/Lead 1.fxp"));
}

/// 音色一覧を開いたまま `Ctrl+Space` を押すと、選択中の音色で入力欄の現在行が鳴る。
///
/// 音色一覧の行は MML を持たないので、鳴らすのは「入力欄側のカーソル行」。
#[test]
fn ctrl_space_in_the_patch_select_plays_the_current_line_with_the_selected_patch() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Down), now);

    let (patch, program) = played(overlay.handle_key(ctrl(KeyCode::Char(' ')), now));

    assert_eq!(
        patch,
        PatchChange::Switch(Some("Pads/Pad 1.fxp".to_string()))
    );
    assert_eq!(
        overlay.line_status(),
        &LineStatus::Played {
            from_chord: false,
            note_count: 3,
        }
    );
    assert!(!program.is_silent());
    // 絞り込み欄へ空白が入っていないこと（一覧が絞られたら試聴どころではない）。
    assert_eq!(overlay.patch_select().unwrap().filtered_len(), 2);
    assert_eq!(overlay.value(), "cde");
}

/// 端末によっては `Ctrl+Space` が `Char('\0')` で届く。音色一覧でも同じに扱う。
#[test]
fn ctrl_space_in_the_patch_select_also_arrives_as_a_nul_char() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    let (patch, _) = played(overlay.handle_key(ctrl(KeyCode::Char('\0')), now));

    assert_eq!(
        patch,
        PatchChange::Switch(Some("Leads/Lead 1.fxp".to_string()))
    );
}

/// `Ctrl+Space` も音源へ音色を読み込ませるので、取り消しは元の音色へ戻す。
///
/// 試聴の記録を進め忘れると、Esc で「読み込ませたまま戻さない」が起きる。
#[test]
fn cancelling_after_ctrl_space_restores_the_patch_it_opened_with() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(ctrl(KeyCode::Char(' ')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::SetPatch {
            patch: None,
            notes: None,
        }
    );
}

/// 演奏設定は overlay 全体で共通。音色一覧からの `Ctrl+Space` にも載る。
#[test]
fn ctrl_space_in_the_patch_select_carries_the_play_settings() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    turn_on_repeat(&mut overlay, now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    let (_, program) = played(overlay.handle_key(ctrl(KeyCode::Char(' ')), now));

    assert!(program.repeat);
}

/// repeat ON なら↑↓でループごと音色を差し替える（1 音プレビューへ落とさない）。
#[test]
fn with_repeat_on_moving_the_patch_cursor_replays_the_whole_line() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    turn_on_repeat(&mut overlay, now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    let (patch, program) = played(overlay.handle_key(press(KeyCode::Down), now));

    assert_eq!(
        patch,
        PatchChange::Switch(Some("Pads/Pad 1.fxp".to_string()))
    );
    assert!(program.repeat);
    assert!(!program.is_silent());
}

/// repeat ON でも鳴らす行が無ければ 1 音の試聴へ戻す。
///
/// ループの代わりに無音になると、音色そのものを聴く手段が消えてしまう。
#[test]
fn with_repeat_on_an_empty_line_still_previews_the_fallback_note() {
    let mut overlay = opened_with_patches();
    let now = Instant::now();
    turn_on_repeat(&mut overlay, now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Down), now),
        MmlOverlayAction::SetPatch {
            patch: Some("Pads/Pad 1.fxp".to_string()),
            notes: Some(NoteRequest {
                messages: vec![[0x90, 60, 127]],
                duration: Duration::from_millis(250),
            }),
        }
    );
}
