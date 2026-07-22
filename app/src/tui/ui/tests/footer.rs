use super::*;

#[test]
fn normal_screen_splits_status_and_keybinds_without_line_numbers() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["abc".to_string()];

    let lines = render_lines(&mut app, 220, 9);
    let screen = lines.join("\n");
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let status_row = lines
        .iter()
        .position(|line| line.trim_start() == "NORMAL")
        .unwrap();
    let render_row = normalized_lines
        .iter()
        .position(|line| line.contains("render:実行0/2予約0"))
        .unwrap();
    let keybind_row = lines
        .iter()
        .position(|line| line.contains("q ?:help e:config b:loops"))
        .unwrap();

    assert!(screen.contains("[NORMAL] notepad mode"));
    assert!(screen.contains("▶   abc"));
    assert!(!screen.contains("MML Lines"));
    assert!(!screen.contains("▶   1 abc"));
    assert_eq!(render_row, status_row + 1);
    assert_eq!(keybind_row, render_row + 1);
    assert!(normalized_lines[render_row].contains("render:実行0/2予約0"));
    assert!(screen.contains("q ?:help e:config b:loops"));
    assert!(screen.contains("b:loops"));
    assert!(screen.contains("dd/Del:cut"));
    assert!(screen.contains("g:generate"));
    assert!(screen.contains("Shift+H:patch history"));
    assert!(!screen.contains("Shift+L:log"));
    assert!(!screen.contains("notepad r log"));
    assert!(!screen.contains("selected list"));
    assert!(screen.contains("w:DAW"));
    assert!(screen.contains("v:keyboard"));
}
