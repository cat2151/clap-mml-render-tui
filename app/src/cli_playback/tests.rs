use super::*;

#[test]
fn converts_single_chord_to_mml() {
    assert_eq!(
        cli_playback_mml("C"),
        CliPlaybackMml::Chord {
            chord: "C".to_string(),
            mml: "v11'c1eg'".to_string(),
        }
    );
}

#[test]
fn converts_chord_progression_to_mml() {
    assert_eq!(
        cli_playback_mml("Dm G7 C"),
        CliPlaybackMml::Chord {
            chord: "Dm G7 C".to_string(),
            mml: "v11'd1fa''g1b<df''c1eg'".to_string(),
        }
    );
}

#[test]
fn keeps_regular_mml_when_chord_parse_fails() {
    assert_eq!(
        cli_playback_mml("cde"),
        CliPlaybackMml::Mml("cde".to_string())
    );
}
