use super::*;

/// パラメータを `params` で埋め、SysEx の末尾に名前を入れた program。
fn program(display: &str, params: u8, name: &str) -> DexedProgram {
    let mut sysex = vec![params; 145];
    sysex.extend_from_slice(format!("{name:<10}").as_bytes());
    DexedProgram {
        display: display.to_string(),
        sysex,
        params: vec![params; 145],
    }
}

#[test]
fn exact_duplicates_are_grouped_largest_first() {
    let programs = vec![
        program("a.syx/00 X", 1, "X"),
        program("b.syx/00 Y", 2, "Y"),
        program("c.syx/00 X", 1, "X"),
        program("d.syx/00 Y", 2, "Y"),
        program("e.syx/00 Y", 2, "Y"),
        program("f.syx/00 Z", 3, "Z"),
    ];

    let groups = duplicate_groups(&programs, exact_key);

    assert_eq!(groups, vec![vec![1, 3, 4], vec![0, 2]]);
}

#[test]
fn same_params_with_different_names_are_only_a_params_duplicate() {
    let programs = vec![
        program("a.syx/00 BELL", 1, "BELL"),
        program("b.syx/00 BELL 2", 1, "BELL 2"),
    ];

    assert!(duplicate_groups(&programs, exact_key).is_empty());
    assert_eq!(duplicate_groups(&programs, params_key), vec![vec![0, 1]]);

    let text = report("bell", &programs, &[]);
    assert!(text.contains("SysEx 完全一致: unique=2 duplicate_groups=0 redundant=0 (0.0%)"));
    assert!(
        text.contains("名前を除くパラメータ一致: unique=1 duplicate_groups=1 redundant=1 (50.0%)")
    );
    assert!(
        text.contains("[名前だけ違う] group 1: 2 programs\n  a.syx/00 BELL\n  b.syx/00 BELL 2\n")
    );
}

#[test]
fn report_lists_exact_groups_and_skips_name_only_section_when_names_match() {
    let programs = vec![
        program("a.syx/21 SPACE SHOT", 1, "SPACE SHOT"),
        program("b.syx/02 SPACE SHOT", 1, "SPACE SHOT"),
        program("c.syx/00 GUNSHOT", 2, "GUNSHOT"),
    ];

    let text = report("syx shot", &programs, &[]);

    assert!(
        text.starts_with("dexed-duplicates condition=\"syx shot\" programs=3 cartridge_errors=0\n")
    );
    assert!(text.contains("SysEx 完全一致: unique=2 duplicate_groups=1 redundant=1 (33.3%)"));
    assert!(text.contains("名前だけ違う program を含む group=0"));
    assert!(text.contains(
        "[SysEx 完全一致] group 1: 2 programs\n  a.syx/21 SPACE SHOT\n  b.syx/02 SPACE SHOT\n"
    ));
    assert!(!text.contains("[名前だけ違う]"));
}
