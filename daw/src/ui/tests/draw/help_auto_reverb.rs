use super::*;

/// `t` の音色選択は自前のヘルプを持たないので、auto reverb のキーは DAW のヘルプから辿る。
#[test]
fn help_shows_auto_reverb_keys_of_the_patch_selector() {
    let mut app = build_test_app();
    app.mode = DawMode::Help;

    let normalized_lines: Vec<String> = render_lines(&app, 160, 52)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect();

    let line = normalized_lines
        .iter()
        .find(|line| line.contains("（音色選択中）E:autoreverbon/off"))
        .unwrap_or_else(|| panic!("lines: {normalized_lines:?}"));
    assert!(line.contains("e:ルール編集"), "line: {line:?}");
    assert!(line.contains("ESCで保存して閉じる"), "line: {line:?}");
}
