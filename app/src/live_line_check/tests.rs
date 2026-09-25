use super::*;

#[test]
fn a_line_is_split_into_its_patch_and_its_performance() {
    let lines = check_lines(&[
        r#"{"Surge XT patch": "patches_factory/Keys/Digi Harpsi.fxp"} cde"#.to_string(),
        "g1".to_string(),
    ])
    .unwrap();

    assert_eq!(
        lines[0].live.patch.patch(),
        Some("patches_factory/Keys/Digi Harpsi.fxp")
    );
    assert!(!lines[0].live.program.performance.is_silent());
    assert_eq!(lines[1].live.patch.patch(), None);
}

#[test]
fn an_empty_line_is_rejected() {
    assert!(check_lines(&["  ".to_string()]).is_err());
    assert!(check_lines(&[]).is_err());
}

#[test]
fn renders_are_summed_at_their_send_frames() {
    let mut mixed = Vec::new();
    mix_at(&mut mixed, &[1.0, 1.0, 1.0, 1.0], 0);
    mix_at(&mut mixed, &[0.5, 0.5, 0.5, 0.5], 1);
    assert_eq!(mixed, vec![1.0, 1.0, 1.5, 1.5, 0.5, 0.5]);
}
