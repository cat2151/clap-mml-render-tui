use super::*;

fn dexed(display: &str) -> PatchRoleInput<'_> {
    PatchRoleInput {
        plugin: Some("Dexed"),
        ..input(display, None)
    }
}

/// Dexedのpiano・bass・strings・brassは数が多いので、同じRoleの中で`FM`付きのlabelへ分ける。
#[test]
fn dexed_piano_bass_strings_and_brass_get_their_own_fm_labels() {
    let surge_piano = "Keys/Grand Piano.fxp";
    let surge_bass = "Basses/Sub Bass.fxp";
    let surge_strings = "Strings/Warm Strings.fxp";
    let surge_brass = "Brass/Big Brass.fxp";
    let dexed_piano = "rom1a.syx/10 e.piano 1";
    let dexed_bass = "rom1a.syx/15 bass    1";
    let dexed_strings = "rom1a.syx/03 strings 1";
    let dexed_brass = "rom1a.syx/00 brass   1";
    let dexed_organ = "rom1a.syx/17 e.organ 1";
    let index = PatchRoleIndex::build(
        [
            PatchRoleInput {
                plugin: Some("Surge XT"),
                ..input(surge_piano, None)
            },
            input(surge_bass, None),
            input(surge_strings, None),
            input(surge_brass, None),
            dexed(dexed_piano),
            dexed(dexed_bass),
            dexed(dexed_strings),
            dexed(dexed_brass),
            dexed(dexed_organ),
        ],
        &[],
    );

    assert_eq!(index.role_of(dexed_piano), Some(PatchRole::Chord));
    assert_eq!(index.preset_label_of(dexed_piano), Some("FM Piano"));
    assert_eq!(index.role_of(dexed_bass), Some(PatchRole::Bass));
    assert_eq!(index.preset_label_of(dexed_bass), Some("FM Bass"));
    assert_eq!(
        index.preset_label_of(surge_piano),
        Some("keyboard|keys|piano")
    );
    assert_eq!(index.preset_label_of(surge_bass), Some("bass|bs"));
    assert_eq!(index.role_of(dexed_strings), Some(PatchRole::Chord));
    assert_eq!(index.preset_label_of(dexed_strings), Some("FM Strings"));
    assert_eq!(index.role_of(dexed_brass), Some(PatchRole::Chord));
    assert_eq!(index.preset_label_of(dexed_brass), Some("FM Brass"));
    assert_eq!(index.preset_label_of(surge_strings), Some("strings"));
    assert_eq!(index.preset_label_of(surge_brass), Some("brass"));
    // FM付きのlabelを持たない楽器は、Dexedでも共通のlabelのまま。
    assert_eq!(index.preset_label_of(dexed_organ), Some("organ"));
}

/// plugin条件はユーザー追加presetでも、selectorの手入力と同じ意味になる。
#[test]
fn user_presets_can_use_plugin_terms() {
    let index = PatchRoleIndex::build(
        [dexed("rom1a.syx/03 theremin"), input("theremin solo", None)],
        &[("lead".to_string(), r"\btheremin plugin:dexed".to_string())],
    );

    assert_eq!(
        index.role_of("rom1a.syx/03 theremin"),
        Some(PatchRole::Lead)
    );
    assert_eq!(index.role_of("theremin solo"), Some(PatchRole::Etc));
}
