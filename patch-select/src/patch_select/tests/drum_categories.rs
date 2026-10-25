use super::*;

#[test]
fn drum_presets_partition_the_role_and_open_on_the_matching_part() {
    let cases = [
        ("DR Hat 001.vvp", "hi-hat"),
        ("DR HiHat 002.vvp", "hi-hat"),
        ("DR Hi-Hat 003.vvp", "hi-hat"),
        ("DR Kick Snare Hat.vvp", "kick"),
        ("DR Snare Hat.vvp", "snare"),
        ("DR Mystery.vvp", "percussion"),
        ("Percussion/Djembe.fxp", "percussion"),
    ];
    for (current, expected) in cases {
        let select = PatchSelect::open(PatchSelectRequest {
            patches: cases
                .iter()
                .map(|(name, _)| entry(name, "Vaporizer2", Some("Drum")))
                .collect(),
            current: Some(current.into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(select.presets()[select.preset_cursor()].label, expected);
        assert_eq!(select.selected(), Some(current));
        let parts: Vec<_> = select
            .presets()
            .iter()
            .filter(|p| p.pattern.is_some())
            .collect();
        assert_eq!(
            parts.iter().map(|p| p.label.as_str()).collect::<Vec<_>>(),
            ["kick", "snare", "hi-hat", "percussion"]
        );
        for (name, label) in cases {
            let memberships: Vec<_> = parts
                .iter()
                .filter(|p| p.matches.iter().any(|&i| select.all[i].display() == name))
                .map(|p| p.label.as_str())
                .collect();
            assert_eq!(memberships, [label], "{name}");
        }
    }
}
