use super::*;

/// 実在チェックはこの関数の外（[`installed_plugin_profiles`]）で済ませてある前提なので、
/// テストは存在しないパスを渡してよい。
fn profile(plugin_path: &str, patches_dirs: &[&str]) -> PluginProfile {
    PluginProfile {
        plugin_path: plugin_path.to_string(),
        plugin_id: None,
        patches_dirs: Some(patches_dirs.iter().map(|dir| dir.to_string()).collect()),
    }
}

/// 実在チェック済みプロファイルの並び（[`installed_plugin_profiles`] が返す形）へ包む。
/// 実在しない dir の記録は、それを見るテストだけが [`InstalledProfile`] を直接組む。
fn installed(profiles: Vec<(String, PluginProfile)>) -> Vec<InstalledProfile> {
    profiles
        .into_iter()
        .map(|(name, profile)| InstalledProfile {
            name,
            profile,
            missing_dirs: Vec::new(),
            resolved_patches: None,
            source_notices: Vec::new(),
            source_error: None,
        })
        .collect()
}

/// カタログに**載ったぶん**だけを見るテスト用の包み。外したぶんを見るテストは
/// [`catalog_plugins_with`] を直接呼ぶ。
fn listed(cfg: &Config, profiles: Vec<(String, PluginProfile)>) -> Vec<CatalogPlugin> {
    catalog_plugins_with(cfg, installed(profiles)).0
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

    let plugins = listed(&cfg, Vec::new());

    assert_eq!(plugins.len(), 1);
    assert!(plugins[0].dirs.is_empty());
    assert_eq!(plugins[0].base, None);
}

#[test]
fn catalog_plugins_uses_a_single_dir_as_its_own_base() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/tmp/patches_factory"]"#);

    let plugins = listed(&cfg, Vec::new());

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

    let plugins = listed(&cfg, Vec::new());

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

    let plugins = listed(&cfg, Vec::new());

    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].base, None);
}

/// 2 つめのプラグインは**自分の dir を基点に**相対化する。プラグインを跨いだ共通の親を
/// 取ると display 文字列（＝永続 ID）が変わり、保存済みデータが指し先を失う。
#[test]
fn catalog_plugins_relativize_each_plugin_against_its_own_base() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/tmp/surge-data/patches_factory"]"#);

    let plugins = listed(
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
///
/// Vaporizer2 の組み込みプロファイルは `patches_dirs` を持たない（プリセット置き場は
/// ユーザーが決めるものなので config に書いてもらう）。config に書かなければ、
/// **インストール済みでもカタログには載らない**という倒れ方をここが決めている。
#[test]
fn catalog_plugins_skip_a_plugin_without_patch_dirs() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/tmp/patches_factory"]"#);

    let plugins = listed(
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
fn catalog_plugins_do_not_list_the_primary_plugin_twice() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/tmp/my-surge-patches"]"#);

    let plugins = listed(
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

/// Vaporizer2 が 3 つめのカタログ項目として並ぶこと。基点は自分の音色置き場から取り、
/// 他プラグインと束ねない（束ねると display 文字列＝永続 ID が壊れる。ADR 0006）。
#[test]
fn a_vaporizer2_profile_becomes_a_third_catalog_plugin() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/opt/surge/patches_factory"]"#);

    let plugins = listed(
        &cfg,
        vec![
            (
                "Dexed".to_string(),
                profile("/usr/lib/clap/Dexed.clap", &["/home/f/dexed/Cartridges"]),
            ),
            (
                "Vaporizer2".to_string(),
                profile(
                    "/usr/lib/clap/VASTvaporizer2.clap",
                    &["/home/f/Vaporizer2/Presets"],
                ),
            ),
        ],
    );

    assert_eq!(plugins.len(), 3);
    assert_eq!(plugins[2].name, "Vaporizer2");
    assert_eq!(
        plugins[2].base.as_deref(),
        Some("/home/f/Vaporizer2/Presets")
    );
}

/// 音色置き場を書いていないプラグインは、カタログから外れたことが理由つきで残る。
/// **Vaporizer2 の組み込みプロファイルがまさにこれ**で、この 1 件が見えないと
/// 「インストールしたのに音色が 1 件も出ない」の原因に誰も辿り着けない。
#[test]
fn a_plugin_without_patch_dirs_is_reported_as_skipped() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/opt/surge/patches_factory"]"#);
    let vaporizer2 = profile("/usr/lib/clap/VASTvaporizer2.clap", &[]);

    let (plugins, skipped) = catalog_plugins_with(
        &cfg,
        installed(vec![("Vaporizer2".to_string(), vaporizer2)]),
    );

    // 載せないという倒れ方そのものは変えていない。
    assert_eq!(plugins.len(), 1);
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].name, "Vaporizer2");
    assert_eq!(skipped[0].reason, CatalogSkipReason::NoPatchDirs);
    assert_eq!(skipped[0].reason_code(), "no-patches-dirs");
    // 案内は「どこへ何を書くか」まで言う。
    let notice = skipped[0].notice_line();
    assert!(notice.contains("Vaporizer2"), "{notice}");
    assert!(notice.contains("[plugins.Vaporizer2]"), "{notice}");
    assert!(notice.contains("patches_dirs"), "{notice}");
}

/// 書いてあるが 1 つも実在しない場合は「未設定」と言わない。綴りを間違えた dir を
/// 名指しで返す（「未設定です」と案内されると、書いてある本人には直しようがない）。
#[test]
fn a_plugin_whose_patch_dirs_all_vanished_is_reported_as_missing() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/opt/surge/patches_factory"]"#);
    let vaporizer2 = profile("/usr/lib/clap/VASTvaporizer2.clap", &[]);

    let (_, skipped) = catalog_plugins_with(
        &cfg,
        vec![InstalledProfile {
            name: "Vaporizer2".to_string(),
            profile: vaporizer2,
            missing_dirs: vec!["/typo/Vaporizer2/Presets".to_string()],
            resolved_patches: None,
            source_notices: Vec::new(),
            source_error: None,
        }],
    );

    assert_eq!(
        skipped[0].reason,
        CatalogSkipReason::PatchDirsMissing(vec!["/typo/Vaporizer2/Presets".to_string()])
    );
    assert_eq!(skipped[0].reason_code(), "patch-dirs-missing");
    assert!(
        skipped[0]
            .notice_line()
            .contains("/typo/Vaporizer2/Presets"),
        "{}",
        skipped[0].notice_line()
    );
}

#[test]
fn adapter_reports_config_and_program_source_failures_together() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/opt/surge/patches_factory"]"#);
    let sforzando = PluginProfile {
        plugin_id: Some(crate::SFORZANDO_PLUGIN_ID.to_string()),
        ..profile("/usr/lib/clap/sforzando.clap", &[])
    };

    let (_, skipped) = catalog_plugins_with(
        &cfg,
        vec![InstalledProfile {
            name: "Sforzando".to_string(),
            profile: sforzando,
            missing_dirs: vec!["/typo/sfz".to_string()],
            resolved_patches: Some(Vec::new()),
            source_notices: Vec::new(),
            source_error: Some("no registered programs".to_string()),
        }],
    );

    assert_eq!(skipped[0].reason_code(), "patch-source-unavailable");
    let notice = skipped[0].notice_line();
    assert!(notice.contains("/typo/sfz"), "{notice}");
    assert!(notice.contains("no registered programs"), "{notice}");
}

#[test]
fn adapter_source_failure_skips_a_plugin_even_when_its_directory_exists() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/opt/surge/patches_factory"]"#);
    let adapter = profile("/usr/lib/clap/adapter.clap", &["/existing/programs"]);

    let (plugins, skipped) = catalog_plugins_with(
        &cfg,
        vec![InstalledProfile {
            name: "adapter".to_string(),
            profile: adapter,
            missing_dirs: Vec::new(),
            resolved_patches: Some(Vec::new()),
            source_notices: Vec::new(),
            source_error: Some("no loadable programs".to_string()),
        }],
    );

    assert_eq!(plugins.len(), 1);
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].reason_code(), "patch-source-unavailable");
}

#[test]
fn adapter_source_notice_is_retained_as_catalog_data() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/opt/surge/patches_factory"]"#);
    let adapter = profile("/usr/lib/clap/adapter.clap", &["/existing/programs"]);

    let (plugins, skipped) = catalog_plugins_with(
        &cfg,
        vec![InstalledProfile {
            name: "adapter".to_string(),
            profile: adapter,
            missing_dirs: Vec::new(),
            resolved_patches: Some(vec!["/existing/programs/voice.patch".into()]),
            source_notices: vec!["11 helper files excluded".to_string()],
            source_error: None,
        }],
    );

    assert!(skipped.is_empty());
    assert_eq!(
        plugins[1].source_notices,
        ["11 helper files excluded".to_string()]
    );
}

/// 音色置き場を持つプラグインは載るので、外した一覧には出ない。
/// 「何でもかんでも外したと言う」実装への番人。
#[test]
fn a_plugin_with_patch_dirs_is_not_reported_as_skipped() {
    let cfg = config_with_patch_dirs(r#"patches_dirs = ["/opt/surge/patches_factory"]"#);
    let dexed = profile("/usr/lib/clap/Dexed.clap", &["/opt/dexed/cartridges"]);

    let (plugins, skipped) =
        catalog_plugins_with(&cfg, installed(vec![("Dexed".to_string(), dexed)]));

    assert_eq!(plugins.len(), 2);
    assert!(skipped.is_empty());
}

/// 既定プラグインは音色置き場が空でも外れない（実在チェックをしない唯一の枠）。
/// 外した一覧にも出ない。出すと「設定を直せば載る」という誤った案内になる。
#[test]
fn the_default_plugin_is_never_reported_as_skipped() {
    let cfg = config_with_patch_dirs("");

    let (plugins, skipped) = catalog_plugins_with(&cfg, Vec::new());

    assert_eq!(plugins.len(), 1);
    assert!(skipped.is_empty());
}
