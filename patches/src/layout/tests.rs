use super::*;

#[test]
fn abstract_metadata_keeps_prefixless_categories() {
    assert_eq!(patch_category("Pads/Warm Pad.fxp"), "Pads");
}

#[test]
fn a_prefix_lookalike_is_a_plain_category() {
    assert_eq!(
        patch_category("patches_factory_backup/Pads/Warm Pad.fxp"),
        "patches_factory_backup"
    );
}

#[test]
fn empty_categories_match_everything() {
    assert!(patch_matches_categories(
        "patches_factory/Drums/Kick.fxp",
        &[]
    ));
    assert!(patch_matches_categories("Dexed_01.syx/00 Say Again.", &[]));
}

#[test]
fn adapter_metadata_expands_derived_categories() {
    assert_eq!(patch_category("AR Accent Arp.vvp"), "Arpeggio");
    assert_eq!(patch_category("patches_factory/Pads/PD Emily.vvp"), "Pad");
}

#[test]
fn existing_categories_remain_stable() {
    assert_eq!(patch_category("patches_factory/Pads/Warm Pad.fxp"), "Pads");
    assert_eq!(patch_category("Dexed_01.syx/00 Say Again."), "Dexed_01.syx");
}

#[test]
fn first_directory_is_available_as_a_neutral_category() {
    assert_eq!(
        patch_category("Celtic Harp Factory Presets/Realistic Celtic Harp.floe-preset"),
        "Celtic Harp Factory Presets"
    );
}
