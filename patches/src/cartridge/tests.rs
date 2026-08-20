use crate::layout::{patch_category, PatchLayout};

/// Dexed の display 文字列はカートリッジ名がカテゴリになる。
/// 用途別の絞り込み（[`crate::selection`]）とグループ表示がこれを前提にしている。
#[test]
fn a_cartridge_file_name_is_the_category() {
    assert_eq!(
        PatchLayout::of("Dexed_01.syx/00 Say Again."),
        PatchLayout::Cartridge
    );
    assert_eq!(patch_category("Dexed_01.syx/00 Say Again."), "Dexed_01.syx");
}

/// カートリッジ dir が 1 段深い形（`SynprezFM/DX7_01.syx/BRASS 1`）でも、
/// 先頭セグメントをカテゴリとして読む。
#[test]
fn a_nested_cartridge_dir_uses_its_first_segment() {
    assert_eq!(patch_category("SynprezFM/DX7_01.syx/BRASS 1"), "SynprezFM");
}
