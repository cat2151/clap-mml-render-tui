use super::*;
use crate::input::HeavyPreview;

fn tone_mml(index: usize) -> String {
    format!(r#"{{"Surge XT patch": "Tone {index:02}"}} l8cdef"#)
}

fn confirming(app: &NotepadScreen<'_>) -> bool {
    matches!(app.heavy_preview, Some(HeavyPreview::Confirm { .. }))
}

fn waiting(app: &NotepadScreen<'_>) -> bool {
    matches!(app.heavy_preview, Some(HeavyPreview::Waiting { .. }))
}

/// カーソルが通っただけの重い音色は鳴らさない。前の音色の音も止める。
#[test]
fn moving_onto_a_heavy_patch_does_not_preview_it() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones_with_heavy(&mut app, 24, "Tone 04", "Tone 05");

    press(&mut app, KeyCode::Char('j'));

    assert_eq!(selected(&app).as_deref(), Some("Tone 05"));
    assert_eq!(playing(&app), None);
    assert!(app.heavy_preview.is_none());
}

#[test]
fn space_on_a_heavy_patch_asks_first_and_n_cancels() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones_with_heavy(&mut app, 24, "Tone 04", "Tone 05");
    press(&mut app, KeyCode::Char('j'));

    press(&mut app, KeyCode::Char(' '));
    assert!(confirming(&app));
    assert_eq!(playing(&app), None);

    press(&mut app, KeyCode::Char('n'));
    assert!(app.heavy_preview.is_none());
    assert_eq!(playing(&app), None);
}

/// y で鳴らし始め、鳴り始めるまではキーを受け付けない。
#[test]
fn yes_renders_the_heavy_patch_and_blocks_keys_until_it_starts() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones_with_heavy(&mut app, 24, "Tone 04", "Tone 05");
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char(' '));

    press(&mut app, KeyCode::Char('y'));
    assert!(waiting(&app));
    assert_eq!(playing(&app), Some(tone_mml(5)));

    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Esc);
    app.pump_heavy_preview_wait();
    assert!(waiting(&app));
    assert_eq!(selected(&app).as_deref(), Some("Tone 05"));

    *app.playback.session.play_state().lock().unwrap() = PlayState::Playing(tone_mml(5));
    app.pump_heavy_preview_wait();
    assert!(app.heavy_preview.is_none());
}

#[test]
fn space_on_a_light_patch_previews_without_asking() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones_with_heavy(&mut app, 24, "Tone 04", "Tone 05");

    press(&mut app, KeyCode::Char(' '));

    assert!(app.heavy_preview.is_none());
    assert_eq!(playing(&app), Some(tone_mml(4)));
}

/// cache にある重い音色は render を待たないので、軽い音色と同じく移動だけで鳴らす。
#[test]
fn moving_onto_a_cached_heavy_patch_previews_it() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones_with_heavy(&mut app, 24, "Tone 04", "Tone 05");
    app.audio
        .known_disk_hashes
        .lock()
        .unwrap()
        .insert(cmrt_history::daw_cache_mml_hash(&tone_mml(5)));

    press(&mut app, KeyCode::Char('j'));

    assert_eq!(playing(&app), Some(tone_mml(5)));
    assert!(app.heavy_preview.is_none());
}

#[test]
fn space_on_a_cached_heavy_patch_previews_without_asking() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones_with_heavy(&mut app, 24, "Tone 04", "Tone 05");
    press(&mut app, KeyCode::Char('j'));
    app.audio
        .cache
        .lock()
        .unwrap()
        .insert(tone_mml(5), vec![0.1, 0.2]);

    press(&mut app, KeyCode::Char(' '));

    assert!(app.heavy_preview.is_none());
    assert_eq!(playing(&app), Some(tone_mml(5)));
}
