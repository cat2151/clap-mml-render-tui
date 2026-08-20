use crate::layout::{patch_category, patch_matches_categories, PatchLayout};

#[test]
fn patch_category_reads_the_factory_layout() {
    assert_eq!(
        PatchLayout::of("patches_factory/Pads/Warm Pad.fxp"),
        PatchLayout::SurgeXt
    );
    assert_eq!(patch_category("patches_factory/Pads/Warm Pad.fxp"), "Pads");
}

#[test]
fn patch_category_skips_the_thirdparty_vendor_segment() {
    assert_eq!(
        patch_category("patches_3rdparty/John/Leads/Sharp Lead.fxp"),
        "Leads"
    );
}

#[test]
fn patch_category_handles_vendorless_thirdparty_paths() {
    assert_eq!(
        patch_category("patches_3rdparty/Leads/Sharp Lead.fxp"),
        "Leads"
    );
}

#[test]
fn patch_matches_categories_ignores_case() {
    let categories = vec!["pads".to_string()];

    assert!(patch_matches_categories(
        "patches_factory/Pads/Warm Pad.fxp",
        &categories
    ));
}

#[test]
fn patch_matches_categories_keeps_singular_and_plural_apart() {
    let categories = vec!["Basses".to_string()];

    assert!(patch_matches_categories(
        "patches_factory/Basses/Deep.fxp",
        &categories
    ));
    // Surge には Bass と Basses が併存する。正規化しないので別カテゴリのまま。
    assert!(!patch_matches_categories(
        "patches_3rdparty/John/Bass/Deep.fxp",
        &categories
    ));
}
