use super::*;

/// MIXER のキーは専用ヘルプにだけ載せ、DAW のヘルプには混ぜない。
#[test]
fn mixer_keys_are_only_in_the_mixer_help() {
    let normalized = |app: &DawApp| -> String {
        render_lines(app, 160, 52)
            .into_iter()
            .map(|line| line.replace(' ', ""))
            .collect::<Vec<_>>()
            .join(
                "
",
            )
    };

    let mut normal = build_test_app();
    normal.mode = DawMode::Help;
    let normal_help = normalized(&normal);
    assert!(!normal_help.contains("MIXERoverlay"), "{normal_help}");
    assert!(!normal_help.contains("volume-/+3dB"), "{normal_help}");

    let mut mixer = build_test_app();
    mixer.mode = DawMode::Help;
    mixer.help_origin = DawMode::Mixer;
    let mixer_help = normalized(&mixer);
    assert!(mixer_help.contains("MIXERoverlay(m)"), "{mixer_help}");
    assert!(mixer_help.contains("?:ヘルプ(このページ)"), "{mixer_help}");
    assert!(mixer_help.contains("j/k,↓/↑:volume-/+3dB"), "{mixer_help}");
    assert!(
        !mixer_help.contains("Shift+H:historyoverlay"),
        "{mixer_help}"
    );
}
