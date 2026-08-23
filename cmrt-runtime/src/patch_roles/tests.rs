use super::*;

#[test]
fn floe_builtin_roles_do_not_filter_any_category_or_keyword() {
    assert_eq!(
        PatchRoles::builtin_for(Some(crate::FLOE_PLUGIN_ID), "Floe.clap"),
        PatchRoles::default()
    );
}

fn top_level_roles() -> PatchRoleFilters {
    PatchRoleFilters {
        chord_patch_categories: Some(vec!["Keys".to_string()]),
        bass_patch_categories: Some(vec!["Bass".to_string()]),
        arpeggio_patch_categories: Some(vec!["Plucks".to_string()]),
        drum_patch_categories: Some(vec!["Percussion".to_string()]),
        kick_patch_keywords: Some(vec!["kick".to_string()]),
        snare_patch_keywords: Some(vec!["snare".to_string()]),
        hihat_patch_keywords: Some(vec!["hat".to_string()]),
    }
}

/// 既定プラグインではないプラグインの config。トップレベルの値は持っている。
fn cfg_with_top_level_roles() -> Config {
    Config {
        plugin_path: r"C:\clap\MySynth.clap".to_string(),
        top_level_patch_roles: top_level_roles(),
        ..Default::default()
    }
}

/// プロファイルが何も書いていなければ、トップレベルの値がそのまま残る（既定プラグイン）。
#[test]
fn unwritten_items_fall_back_to_the_top_level_values() {
    let cfg = cfg_with_top_level_roles();

    let roles = PatchRoles::resolve_for_default_plugin(&cfg, &PatchRoleFilters::default());

    assert_eq!(roles.chord_patch_categories, vec!["Keys".to_string()]);
    assert_eq!(roles.kick_patch_keywords, vec!["kick".to_string()]);
}

/// 明示的な `[]` は「絞らない」なので、トップレベルへ落ちてはいけない（Dexed の形）。
#[test]
fn an_explicit_empty_list_stays_empty_instead_of_falling_back() {
    let cfg = cfg_with_top_level_roles();

    let roles = PatchRoles::resolve_for_default_plugin(&cfg, &PatchRoleFilters::unfiltered());

    assert_eq!(roles, PatchRoles::default());
}

#[test]
fn a_written_item_overrides_only_itself() {
    let cfg = cfg_with_top_level_roles();
    let from_profile = PatchRoleFilters {
        chord_patch_categories: Some(vec!["FM Keys".to_string()]),
        ..Default::default()
    };

    let roles = PatchRoles::resolve_for_default_plugin(&cfg, &from_profile);

    assert_eq!(roles.chord_patch_categories, vec!["FM Keys".to_string()]);
    assert_eq!(roles.bass_patch_categories, vec!["Bass".to_string()]);
}

/// 論点 4 の本体。トップレベルの値（Surge のカテゴリ名）は既定プラグインにだけ効く。
/// カタログに並ぶ他のプラグインの土台にすると、そのプラグインの候補が全滅する。
#[test]
fn the_top_level_values_do_not_reach_another_plugin_in_the_catalog() {
    let cfg = cfg_with_top_level_roles();

    let mine = PatchRoles::resolve(
        &PatchRoleFilters::default(),
        &PatchRoles::builtin_for(None, r"C:\clap\MySynth.clap"),
    );

    assert_eq!(mine, PatchRoles::default());
    // 既定プラグインとしてなら、同じ config でトップレベルの値が効く。
    assert_eq!(
        PatchRoles::resolve_for_default_plugin(&cfg, &PatchRoleFilters::default())
            .chord_patch_categories,
        vec!["Keys".to_string()]
    );
}

/// 組み込みに無いプラグインの既定は「絞らない」。カテゴリ階層があるとは限らないため。
#[test]
fn an_unknown_plugin_defaults_to_no_narrowing() {
    let builtin = PatchRoles::builtin_for(Some("com.example.my-synth"), r"C:\clap\MySynth.clap");

    assert_eq!(builtin, PatchRoles::default());
}

/// Surge XT の既定はカテゴリ名を持つ。config へ 1 文字も書かなくても効く。
#[test]
fn surge_xt_defaults_come_from_the_builtin_not_from_the_config() {
    let builtin = PatchRoles::builtin_for(Some(crate::SURGE_XT_PLUGIN_ID), "");

    assert_eq!(
        builtin.chord_patch_categories,
        cmrt_patches::surge_xt::DEFAULT_CHORD_PATCH_CATEGORY_NAMES
            .iter()
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>()
    );
    assert!(!builtin.kick_patch_keywords.is_empty());
}

/// Vaporizer2 の既定もカテゴリ名を持つ。**Surge のぶんを流用してはいけない。**
#[test]
fn vaporizer2_defaults_come_from_its_own_category_table() {
    let builtin = PatchRoles::builtin_for(Some(crate::VAPORIZER2_PLUGIN_ID), "");

    assert_eq!(
        builtin.chord_patch_categories,
        cmrt_patches::vaporizer2::DEFAULT_CHORD_PATCH_CATEGORY_NAMES
            .iter()
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(builtin.bass_patch_categories, vec!["Bass".to_string()]);
}

/// **綴りが Surge と違う**ことの番人。Vaporizer2 のカテゴリは単数形（`Pad` / `Bass`）で、
/// Surge の複数形（`Pads` / `Basses`）とは 1 つも一致しない。ここが揃ってしまうと
/// 「Surge のぶんをコピーしただけ」の間違いに気づけない。
#[test]
fn the_vaporizer2_categories_are_not_the_surge_ones() {
    let surge = PatchRoles::builtin_for(Some(crate::SURGE_XT_PLUGIN_ID), "");
    let vaporizer2 = PatchRoles::builtin_for(Some(crate::VAPORIZER2_PLUGIN_ID), "");

    assert_ne!(
        surge.chord_patch_categories,
        vaporizer2.chord_patch_categories
    );
    assert_ne!(
        surge.bass_patch_categories,
        vaporizer2.bass_patch_categories
    );
    // drum の振り分けキーワードだけは共有している（太鼓の一般名なので）。
    assert_eq!(surge.kick_patch_keywords, vaporizer2.kick_patch_keywords);
}

/// `plugin_id` を書かない config でも、ファイル名で Vaporizer2 だと分かる。
/// **既定 `plugin_path` との一致ではなくファイル名に `vaporizer` を含むかで見る**ので、
/// 標準以外の場所へ入れていても当たる。
#[test]
fn vaporizer2_is_recognized_by_its_file_name_when_the_id_is_missing() {
    let builtin = PatchRoles::builtin_for(None, r"D:\my\clap\VASTvaporizer2.clap");

    assert_eq!(builtin.bass_patch_categories, vec!["Bass".to_string()]);
}

/// `plugin_id` を書かない旧 config でも、ファイル名で Surge XT だと分かる。
#[test]
fn surge_xt_is_recognized_by_its_file_name_when_the_id_is_missing() {
    let builtin = PatchRoles::builtin_for(None, crate::default_plugin_path());

    assert!(!builtin.chord_patch_categories.is_empty());
}

/// 層を畳む向き。上の層の「書かれている項目」だけが下を隠す。
#[test]
fn layering_keeps_the_unwritten_items_of_the_upper_layer_transparent() {
    let over = PatchRoleFilters {
        chord_patch_categories: Some(vec!["Over".to_string()]),
        bass_patch_categories: Some(Vec::new()),
        ..Default::default()
    };

    let layered = layered_patch_role_filters(&over, &top_level_roles());

    assert_eq!(
        layered.chord_patch_categories,
        Some(vec!["Over".to_string()])
    );
    // 明示の `[]` は上の層の答えなので、下の層へ落ちない。
    assert_eq!(layered.bass_patch_categories, Some(Vec::new()));
    // 書かれていない項目は下の層が見える。
    assert_eq!(
        layered.arpeggio_patch_categories,
        Some(vec!["Plucks".to_string()])
    );
}
