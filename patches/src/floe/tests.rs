use crate::{group_patch_pairs_by_category, sort_patch_pairs, PatchSortOrder};

use super::*;

#[test]
fn floe_extension_is_case_insensitive_and_requires_a_stem() {
    assert!(has_floe_preset_extension("Harp/Realistic.floe-preset"));
    assert!(has_floe_preset_extension("Harp\\Realistic.FLOE-PRESET"));
    assert!(!has_floe_preset_extension(".floe-preset"));
    assert!(!has_floe_preset_extension("Library.floe-pkg"));
}

#[test]
fn first_directory_is_the_category() {
    let pairs = vec![
        pair("Taiko Drums Factory Presets/Taiko Beat 2.floe-preset"),
        pair("Celtic Harp Factory Presets/Harp Trio.floe-preset"),
        pair("Taiko Drums Factory Presets/Taiko Beat.floe-preset"),
    ];
    let categories = group_patch_pairs_by_category(&pairs);

    assert_eq!(categories[0].name, "Celtic Harp Factory Presets");
    assert_eq!(categories[1].name, "Taiko Drums Factory Presets");
    assert_eq!(categories[1].patches.len(), 2);
}

#[test]
fn files_inside_a_floe_category_use_natural_sort() {
    let mut pairs = vec![
        pair("Taiko/Taiko Beat 10.floe-preset"),
        pair("Taiko/Taiko Beat 2.floe-preset"),
        pair("Taiko/Taiko Beat 1.floe-preset"),
    ];
    sort_patch_pairs(&mut pairs, PatchSortOrder::Path);

    assert_eq!(pairs[0].0, "Taiko/Taiko Beat 1.floe-preset");
    assert_eq!(pairs[1].0, "Taiko/Taiko Beat 2.floe-preset");
    assert_eq!(pairs[2].0, "Taiko/Taiko Beat 10.floe-preset");
}

#[test]
fn installed_fixture_shape_groups_as_four_four_three_two() {
    let pairs = [
        "Celtic Harp Factory Presets/Harp Arpeggios.floe-preset",
        "Celtic Harp Factory Presets/Harp Choirpad.floe-preset",
        "Celtic Harp Factory Presets/Harp Trio.floe-preset",
        "Celtic Harp Factory Presets/Realistic Celtic Harp.floe-preset",
        "Ocarina Factory Presets/Ocarina - Polyphonic.floe-preset",
        "Ocarina Factory Presets/Ocarina Machine.floe-preset",
        "Ocarina Factory Presets/Ocarina Mist.floe-preset",
        "Ocarina Factory Presets/Real Ocarina.floe-preset",
        "Taiko Drums Factory Presets/Basic Taiko Drums.floe-preset",
        "Taiko Drums Factory Presets/Taiko Beat 2.floe-preset",
        "Taiko Drums Factory Presets/Taiko Beat.floe-preset",
        "Xylophone Factory Presets/Realistic Xylophone.floe-preset",
        "Xylophone Factory Presets/Sequenced Bars.floe-preset",
    ]
    .into_iter()
    .map(pair)
    .collect::<Vec<_>>();

    let categories = group_patch_pairs_by_category(&pairs);

    assert_eq!(
        categories
            .iter()
            .map(|category| (category.name.as_str(), category.patches.len()))
            .collect::<Vec<_>>(),
        vec![
            ("Celtic Harp Factory Presets", 4),
            ("Ocarina Factory Presets", 4),
            ("Taiko Drums Factory Presets", 3),
            ("Xylophone Factory Presets", 2),
        ]
    );
}

fn pair(display: &str) -> (String, String) {
    (display.to_string(), display.to_lowercase())
}
