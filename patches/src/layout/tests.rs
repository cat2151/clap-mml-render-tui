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

/// `.vvp` は**必ず** Vaporizer2 の読み方へ落ちる。Surge の prefix の下に音色置き場を
/// 置いていても拡張子が勝つ。ここが逆転すると、`.vvp` が Surge のカテゴリ階層で
/// 読まれて用途別の候補から全部外れる。
#[test]
fn a_vvp_patch_always_reads_as_vaporizer2() {
    assert_eq!(
        PatchLayout::of("AR Accent Arp.vvp"),
        PatchLayout::Vaporizer2
    );
    assert_eq!(
        PatchLayout::of("patches_factory/Pads/PD Emily.vvp"),
        PatchLayout::Vaporizer2
    );
    assert_eq!(
        PatchLayout::of("/Vaporizer2/PD Emily.vvp/"),
        PatchLayout::Vaporizer2
    );
}

/// 3 つめの体系を足しても、既存 2 つの読み方は 1 つも変わらない。
#[test]
fn the_existing_layouts_are_untouched_by_the_third_one() {
    assert_eq!(
        PatchLayout::of("patches_factory/Pads/Warm Pad.fxp"),
        PatchLayout::SurgeXt
    );
    assert_eq!(
        PatchLayout::of("Dexed_01.syx/00 Say Again."),
        PatchLayout::Cartridge
    );
    assert_eq!(patch_category("Dexed_01.syx/00 Say Again."), "Dexed_01.syx");
}
