use super::*;

mod favorites;
mod filter;
mod navigation;
mod overlay_switch;
mod prefetch;
mod preview;
mod selection;
mod startup;

/// Role 分類に掛からない（Etc の `ALL` で開く）音色名。名前順に並ぶよう 2 桁にする。
fn tones(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("Tone {index:02}")).collect()
}

fn tone_line(patch: &str) -> String {
    format!(r#"{{"Surge XT patch":"{patch}"}} l8cdef"#)
}

fn open_tones(app: &mut NotepadScreen<'_>, count: usize, current: &str) {
    let names = tones(count);
    let names = names.iter().map(String::as_str).collect::<Vec<_>>();
    open_patch_select_for_test(app, &tone_line(current), &names);
}

fn press(app: &mut NotepadScreen<'_>, code: KeyCode) {
    app.handle_patch_select(KeyEvent::new(code, KeyModifiers::NONE));
}

fn press_ctrl(app: &mut NotepadScreen<'_>, c: char) {
    app.handle_patch_select(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}

fn type_text(app: &mut NotepadScreen<'_>, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
}

fn selected(app: &NotepadScreen<'_>) -> Option<String> {
    app.patch_select_selected_patch_name()
}

fn playing(app: &NotepadScreen<'_>) -> Option<String> {
    match &*app.playback.session.play_state().lock().unwrap() {
        PlayState::Running(mml) => Some(mml.clone()),
        _ => None,
    }
}

fn preset_label(app: &NotepadScreen<'_>) -> String {
    let select = app.patch_select.as_ref().expect("patch select is open");
    select.presets()[select.preset_cursor()].label.clone()
}

fn regex_text(app: &NotepadScreen<'_>) -> String {
    let select = app.patch_select.as_ref().expect("patch select is open");
    cmrt_tui_core::text_input::textarea_value(select.query_textarea())
}

/// Preset pane へ移り、今の Role の `★ Favorite`（preset 1）を選ぶ。
fn choose_favorite_preset(app: &mut NotepadScreen<'_>) {
    press(app, KeyCode::Left);
    press(app, KeyCode::Home);
    press(app, KeyCode::Down);
    press(app, KeyCode::Right);
    assert_eq!(preset_label(app), "★ Favorite");
}

fn add_favorite(app: &mut NotepadScreen<'_>, patch: &str, phrase: &str) {
    app.patch_phrase_store.patches.insert(
        patch.to_string(),
        cmrt_history::PatchPhraseState {
            history: vec![],
            favorites: vec![phrase.to_string()],
        },
    );
}
