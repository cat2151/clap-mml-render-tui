use super::*;
use cmrt_runtime::{PatchBase, PluginProfile, SFORZANDO_PLUGIN_ID};

/// TableWarp2 に同梱される factory preset（`.ariax`）の数。
const TABLEWARP2_FACTORY_PRESETS: usize = 36;

/// 実機の Sforzando を `[plugins.Sforzando]` に書いた Config から、catalog の Sforzando と
/// `build-patch-catalog-cache` が catalog.json に書く `catalog_notes` を作る。
fn installed_sforzando_catalog(
    clap: &str,
    patches_dirs: Option<Vec<String>>,
) -> (CatalogPlugin, Vec<String>) {
    let mut cfg: Config = toml::from_str(&cmrt_runtime::default_config_content()).unwrap();
    cfg.plugins.insert(
        "Sforzando".to_string(),
        PluginProfile {
            plugin_path: clap.to_string(),
            plugin_id: Some(SFORZANDO_PLUGIN_ID.to_string()),
            patches_dirs,
        },
    );
    cmrt_runtime::apply_primary_plugin_profile(&mut cfg).unwrap();
    let (plugins, skipped) = cmrt_runtime::catalog_plugins_detailed(&cfg);
    let notes = catalog_notes(&plugins, &skipped);
    let sforzando = plugins
        .into_iter()
        .find(|plugin| plugin.plugin_id.as_deref() == Some(SFORZANDO_PLUGIN_ID))
        .expect("Sforzando should be listed in the catalog");
    (sforzando, notes)
}

fn listing_pairs(plugin: &CatalogPlugin) -> Vec<(String, String)> {
    cmrt_tui_core::patches::collect_patch_listing_from_catalog(std::slice::from_ref(plugin))
        .unwrap()
        .pairs
}

/// `build-patch-catalog-cache` が catalog.json に書く Sforzando の display を、実機の registry で確かめる。
/// patch load の計測（play server の起動）は含めない。
/// `CMRT_TEST_SFORZANDO_DISPLAYS_OUT` を渡すと display の一覧を 1 行 1 件で書き出す。
#[test]
#[ignore = "installed Sforzando is required: set CMRT_TEST_SFORZANDO_CLAP"]
fn installed_sforzando_displays_start_with_their_root_folder_name() {
    let Some(clap) = std::env::var("CMRT_TEST_SFORZANDO_CLAP").ok() else {
        eprintln!("skip: CMRT_TEST_SFORZANDO_CLAP is not set");
        return;
    };
    let (sforzando, _) = installed_sforzando_catalog(&clap, None);
    let PatchBase::PerRoot(roots) = &sforzando.base else {
        panic!("Sforzando base should be per root: {:?}", sforzando.base);
    };
    let root_names: Vec<String> = roots
        .iter()
        .map(|root| {
            Path::new(root)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    let pairs = listing_pairs(&sforzando);
    let mut per_root = BTreeMap::<String, usize>::new();
    for (display, _) in &pairs {
        let first = display.split('/').next().unwrap();
        assert!(
            root_names.iter().any(|name| name == first),
            "display should start with a root folder name {root_names:?}: {display}"
        );
        assert!(
            !Path::new(display).is_absolute() && !display.contains(':'),
            "display should be relative: {display}"
        );
        assert!(
            Path::new(&sforzando.base.resolve(display)).is_file(),
            "display should resolve to an existing file: {display}"
        );
        *per_root.entry(first.to_string()).or_default() += 1;
    }
    let has_extension = |display: &str, extension: &str| {
        Path::new(display)
            .extension()
            .is_some_and(|found| found.eq_ignore_ascii_case(extension))
    };
    let tablewarp2_presets = pairs
        .iter()
        .filter(|(display, _)| {
            display.starts_with("TableWarp2/") && has_extension(display, "ariax")
        })
        .count();
    let free_sounds_programs = pairs
        .iter()
        .filter(|(display, _)| display.starts_with("Free Sounds/") && has_extension(display, "sfz"))
        .count();
    eprintln!(
        "sforzando displays={} per_root={per_root:?} tablewarp2_ariax={tablewarp2_presets} free_sounds_sfz={free_sounds_programs}",
        pairs.len()
    );
    if let Some(out) = std::env::var_os("CMRT_TEST_SFORZANDO_DISPLAYS_OUT") {
        let lines: Vec<&str> = pairs.iter().map(|(display, _)| display.as_str()).collect();
        std::fs::write(out, lines.join("\n")).unwrap();
    }
    assert_eq!(tablewarp2_presets, TABLEWARP2_FACTORY_PRESETS);
    assert!(
        pairs
            .iter()
            .any(|(display, _)| display
                == "TableWarp2/Presets/com.Plogue.Aria/Keys/Airy Bells.ariax"),
        "Airy Bells should be listed"
    );
    assert!(
        free_sounds_programs > 0,
        "Free Sounds programs should be listed"
    );
    assert_eq!(
        pairs.len(),
        sforzando.resolved_patches.as_ref().unwrap().len()
    );

    let plugins = std::slice::from_ref(&sforzando);
    let audio = describe_patches(plugins, &pairs).unwrap();
    assert_eq!(audio.len(), pairs.len());

    let cached: CachedPlugin =
        serde_json::from_str(&serde_json::to_string(&CachedPlugin::from(&sforzando)).unwrap())
            .unwrap();
    assert_eq!(CatalogPlugin::from(cached).base, sforzando.base);
}

/// `[plugins.Sforzando] patches_dirs` が config に残っていても、Sforzando の音色と
/// catalog.json の `catalog_notes` は書かないときと同じになる。
#[test]
#[ignore = "installed Sforzando is required: set CMRT_TEST_SFORZANDO_CLAP"]
fn installed_sforzando_ignores_configured_patches_dirs() {
    let Some(clap) = std::env::var("CMRT_TEST_SFORZANDO_CLAP").ok() else {
        eprintln!("skip: CMRT_TEST_SFORZANDO_CLAP is not set");
        return;
    };
    let missing = std::env::temp_dir().join(format!(
        "cmrt-missing-sforzando-patches-dir-{}",
        std::process::id()
    ));
    assert!(!missing.exists());

    let (without, without_notes) = installed_sforzando_catalog(&clap, None);
    let (with, with_notes) =
        installed_sforzando_catalog(&clap, Some(vec![missing.to_string_lossy().into_owned()]));
    eprintln!(
        "sforzando patches={} catalog_notes={without_notes:?}",
        without.resolved_patches.as_ref().map_or(0, Vec::len)
    );

    assert_eq!(with.dirs, without.dirs);
    assert_eq!(with.base, without.base);
    assert_eq!(with.resolved_patches, without.resolved_patches);
    assert_eq!(with.source_notices, without.source_notices);
    assert_eq!(listing_pairs(&with), listing_pairs(&without));
    assert_eq!(with_notes, without_notes);
}
