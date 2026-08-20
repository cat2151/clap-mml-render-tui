use super::*;

/// prefix 抜きで保存された Surge の名前はカートリッジ側の読み方へ落ちるが、
/// どちらも先頭セグメントをカテゴリとして読むので結果は変わらない。
/// **この同値が崩れると、保存済みの patch 名がカテゴリを失う。**
#[test]
fn a_prefixless_surge_name_reads_the_same_either_way() {
    assert_eq!(PatchLayout::of("Pads/Warm Pad.fxp"), PatchLayout::Cartridge);
    assert_eq!(patch_category("Pads/Warm Pad.fxp"), "Pads");
    assert_eq!(patch_path_sort_parts("Pads/Warm Pad.fxp").0, 0);
}

/// prefix は先頭セグメントとして完全に一致したときだけ効く。
#[test]
fn a_prefix_lookalike_is_not_the_surge_layout() {
    assert_eq!(
        PatchLayout::of("patches_factory_backup/Pads/Warm Pad.fxp"),
        PatchLayout::Cartridge
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
