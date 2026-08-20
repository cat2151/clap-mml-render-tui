use super::*;

/// 実在チェックはこの関数の外（[`installed_plugin_profiles`]）で済ませてある前提なので、
/// テストは存在しないパスを渡してよい。
fn profile(plugin_path: &str, patches_dirs: &[&str]) -> PluginProfile {
    PluginProfile {
        plugin_path: plugin_path.to_string(),
        plugin_id: None,
        patches_dirs: Some(patches_dirs.iter().map(|dir| dir.to_string()).collect()),
        patch_roles: crate::PatchRoleFilters::default(),
    }
}

fn config_with_patch_dirs(patches_dirs_line: &str) -> Config {
    let toml_str = format!(
        r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
{patches_dirs_line}
"#
    );
    toml::from_str(&toml_str).unwrap()
}

#[test]
fn catalog_plugins_has_no_dirs_when_none_are_configured() {
    let cfg = config_with_patch_dirs("");

    let plugins = catalog_plugins_with(&cfg, Vec::new());

    assert_eq!(plugins.len(), 1);
    assert!(plugins[0].dirs.is_empty());
    assert_eq!(plugins[0].base, None);
}

#[test]
fn catalog_plugins_uses_a_single_dir_as_its_own_base() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/tmp/patches_factory"]"#);

    let plugins = catalog_plugins_with(&cfg, Vec::new());

    assert_eq!(plugins[0].base.as_deref(), Some("/tmp/patches_factory"));
    assert_eq!(plugins[0].dirs, vec!["/tmp/patches_factory".to_string()]);
}

/// 同じプラグインの複数 dir は基点を共有し、それは共通の親。
/// `patches_factory/...` / `patches_3rdparty/...` という display の先頭は、これが根拠。
#[test]
fn catalog_plugins_share_one_base_across_dirs_of_the_same_plugin() {
    let cfg = config_with_patch_dirs(
        r#"patches_dirs = ["/tmp/surge-data/patches_factory", "/tmp/surge-data/patches_3rdparty"]"#,
    );

    let plugins = catalog_plugins_with(&cfg, Vec::new());

    assert_eq!(plugins[0].base.as_deref(), Some("/tmp/surge-data"));
    assert_eq!(
        plugins[0].dirs,
        vec![
            "/tmp/surge-data/patches_factory".to_string(),
            "/tmp/surge-data/patches_3rdparty".to_string(),
        ]
    );
}

/// 共通の親が根まで登っても見つからないときは相対化しない。
#[test]
fn catalog_plugins_have_no_base_when_dirs_share_no_parent() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["patches_factory", "patches_3rdparty"]"#);

    let plugins = catalog_plugins_with(&cfg, Vec::new());

    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].base, None);
}

/// 用途別絞り込みは、トップレベルの値をプロファイルの差分で解決したもの。
#[test]
fn catalog_plugins_resolve_patch_roles_from_the_top_level_values() {
    let mut cfg = config_with_patch_dirs("");
    cfg.top_level_patch_roles.chord_patch_categories = Some(vec!["Keys".to_string()]);
    cfg.active_patch_roles = crate::PatchRoleFilters {
        bass_patch_categories: Some(Vec::new()),
        ..Default::default()
    };

    let plugins = catalog_plugins_with(&cfg, Vec::new());

    assert_eq!(
        plugins[0].patch_roles.chord_patch_categories,
        vec!["Keys".to_string()]
    );
    assert!(plugins[0].patch_roles.bass_patch_categories.is_empty());
}

/// 2 つめのプラグインは**自分の dir を基点に**相対化する。プラグインを跨いだ共通の親を
/// 取ると display 文字列（＝永続 ID）が変わり、保存済みデータが指し先を失う。
#[test]
fn catalog_plugins_relativize_each_plugin_against_its_own_base() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/tmp/surge-data/patches_factory"]"#);

    let plugins = catalog_plugins_with(
        &cfg,
        vec![(
            "Dexed".to_string(),
            profile("/usr/lib/clap/Dexed.clap", &["/home/f/dexed/Cartridges"]),
        )],
    );

    assert_eq!(plugins.len(), 2);
    assert_eq!(
        plugins[0].base.as_deref(),
        Some("/tmp/surge-data/patches_factory")
    );
    assert_eq!(plugins[1].name, "Dexed");
    assert_eq!(plugins[1].base.as_deref(), Some("/home/f/dexed/Cartridges"));
}

/// 音色置き場が 1 つも実在しないプラグインはカタログへ載せない。
/// 載せると `read_dir` の `Err` で一覧の収集そのものが失敗する。
#[test]
fn catalog_plugins_skip_a_plugin_without_patch_dirs() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/tmp/patches_factory"]"#);

    let plugins = catalog_plugins_with(
        &cfg,
        vec![(
            "Dexed".to_string(),
            profile("/usr/lib/clap/Dexed.clap", &[]),
        )],
    );

    assert_eq!(plugins.len(), 1);
}

/// 既定プラグインと同じプラグインのプロファイルは二重に載せない。
/// 既定は焼き込み済みで名前が残らないので、突き合わせはプラグインの同一性で行う。
#[test]
fn catalog_plugins_do_not_list_the_active_plugin_twice() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/tmp/my-surge-patches"]"#);

    let plugins = catalog_plugins_with(
        &cfg,
        vec![(
            "Surge XT".to_string(),
            profile(
                "/usr/lib/clap/Surge XT.clap",
                &["/opt/surge/patches_factory"],
            ),
        )],
    );

    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].dirs, vec!["/tmp/my-surge-patches".to_string()]);
}

/// 用途別絞り込みはプラグインごとに解決する。トップレベルの値（＝既定プラグイン用の
/// レガシー綴り）が 2 つめのプラグインへ漏れない。
#[test]
fn catalog_plugins_resolve_patch_roles_per_plugin() {
    let mut cfg = config_with_patch_dirs("");
    cfg.top_level_patch_roles.chord_patch_categories = Some(vec!["Keys".to_string()]);

    let mut dexed = profile("/usr/lib/clap/Dexed.clap", &["/home/f/dexed/Cartridges"]);
    dexed.patch_roles = crate::PatchRoleFilters::unfiltered();
    let plugins = catalog_plugins_with(&cfg, vec![("Dexed".to_string(), dexed)]);

    assert_eq!(
        plugins[0].patch_roles.chord_patch_categories,
        vec!["Keys".to_string()]
    );
    assert!(plugins[1].patch_roles.chord_patch_categories.is_empty());
}

/// `active_plugin` を書かない config でも、既定プラグインと同じものを指す `[plugins.*]` に
/// 書いた用途別絞り込みは効く。
///
/// この経路では `apply_active_plugin_profile` が動かない（`active_plugin` が無いので
/// `Ok(None)` で戻る）ため、重複排除でプロファイルを丸ごと捨てると、書いた設定が
/// **黙って無視される**。音色置き場は既定側が正なので捨てるが、絞り込みだけは拾う。
#[test]
fn a_profile_for_the_default_plugin_still_contributes_its_patch_roles() {
    let cfg = config_with_patch_dirs("");
    let mut surge = profile(
        "/usr/lib/clap/Surge XT.clap",
        &["/opt/surge/patches_factory"],
    );
    surge.patch_roles.chord_patch_categories = Some(vec!["MyPads".to_string()]);

    let plugins = catalog_plugins_with(&cfg, vec![("Surge XT".to_string(), surge)]);

    // 二重には載らない。
    assert_eq!(plugins.len(), 1);
    assert_eq!(
        plugins[0].patch_roles.chord_patch_categories,
        vec!["MyPads".to_string()]
    );
    // 書かなかった項目は Surge XT の組み込み既定のまま。
    assert_eq!(
        plugins[0].patch_roles.bass_patch_categories,
        crate::PatchRoles::builtin_for(Some(crate::SURGE_XT_PLUGIN_ID), "").bass_patch_categories
    );
}

/// 既定プラグインが Surge XT なら、config へ 1 文字も書かなくてもカテゴリで絞られる。
/// 既定値の置き場がトップレベルから組み込みへ移ったあとの担保。
#[test]
fn the_default_surge_plugin_narrows_even_with_an_empty_config() {
    let cfg = config_with_patch_dirs("");

    let plugins = catalog_plugins_with(&cfg, Vec::new());

    assert!(!plugins[0].patch_roles.chord_patch_categories.is_empty());
    assert!(!plugins[0].patch_roles.kick_patch_keywords.is_empty());
}
