use super::*;

fn colored(line: &str) -> Vec<(&str, SyntaxKind)> {
    highlight(line)
        .into_iter()
        .filter_map(|(range, kind)| Some((&line[range], kind?)))
        .collect()
}

#[test]
fn segments_cover_the_whole_line_in_order() {
    let line = r#"{"Surge XT patch": "Pads/Pad 1.fxp"} l8c+d'eg'r"#;
    let segments = highlight(line);
    let mut pos = 0;
    for (range, _) in &segments {
        assert_eq!(range.start, pos);
        pos = range.end;
    }
    assert_eq!(pos, line.len());
}

#[test]
fn patch_name_is_split_into_stem_and_the_rest() {
    let line = r#"{"Surge XT patch": "Pads/Pad 1.fxp"} r"#;
    assert_eq!(
        colored(line),
        vec![
            ("\"Pads/", SyntaxKind::PatchPath),
            ("Pad 1", SyntaxKind::PatchStem),
            (".fxp\"", SyntaxKind::PatchPath),
        ]
    );
}

#[test]
fn filter_query_is_not_colored_as_patch_name() {
    let line = r#"{"Surge XT patch": "Pad 1.fxp", "Surge XT patch filter": "pad"} r"#;
    assert_eq!(
        colored(line),
        vec![
            ("\"", SyntaxKind::PatchPath),
            ("Pad 1", SyntaxKind::PatchStem),
            (".fxp\"", SyntaxKind::PatchPath),
        ]
    );
}

#[test]
fn effect_preset_names_in_chain_and_auto_reverb_are_colored() {
    let line = format!(
        r#"{{"Surge XT patch": "P.fxp", "{EFFECT_CHAIN_JSON_KEY}": [{{"Dragonfly Room Reverb preset": "Small Drum Room", "bypass": true}}], "{AUTO_REVERB_JSON_KEY}": {{"Dragonfly Room Reverb preset": "Small Drum Room"}}}} r"#
    );
    let presets: Vec<&str> = colored(&line)
        .into_iter()
        .filter(|(_, kind)| *kind == SyntaxKind::EffectPreset)
        .map(|(text, _)| text)
        .collect();
    assert_eq!(presets, vec!["\"Small Drum Room\"", "\"Small Drum Room\""]);
}

#[test]
fn only_note_names_in_the_mml_body_are_colored() {
    let line = r#"{"Surge XT patch": "abc.fxp"} t120 o4 l8 c+4. r 'eg' b"#;
    let notes: Vec<&str> = colored(line)
        .into_iter()
        .filter(|(_, kind)| *kind == SyntaxKind::NoteName)
        .map(|(text, _)| text)
        .collect();
    assert_eq!(notes, vec!["c", "e", "g", "b"]);
}

#[test]
fn line_without_json_colors_notes_only() {
    assert_eq!(
        colored("cde"),
        vec![
            ("c", SyntaxKind::NoteName),
            ("d", SyntaxKind::NoteName),
            ("e", SyntaxKind::NoteName),
        ]
    );
}
