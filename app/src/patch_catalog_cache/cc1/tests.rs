use super::*;
use std::{fs, path::PathBuf, time::Instant};

#[test]
#[ignore = "requires an isolated CMRT_CC1_TEST_CATALOG copied from the installed catalog"]
fn rebuild_full_catalog() {
    let path = fixture_path();
    let cache: CacheFile = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let expected = cache.patches.len();
    assert!(expected > 12);
    let mut plugins: Vec<cmrt_runtime::CatalogPlugin> =
        cache.plugins.into_iter().map(Into::into).collect();

    for plugin in &mut plugins {
        let key =
            cmrt_core::PluginKey::from_identity(plugin.plugin_id.as_deref(), &plugin.plugin_path);
        plugin.resolved_patches = Some(
            cache
                .patches
                .iter()
                .map(|patch| &patch.audio.reference)
                .filter(|patch| patch.plugin == key)
                .map(|patch| {
                    let wire = patch.wire();
                    let wire = cmrt_core::parse_patch_wire(&wire).unwrap();
                    PathBuf::from(plugin.base.resolve(wire.path))
                })
                .collect(),
        );
    }
    let listing = cmrt_tui_core::patches::collect_patch_listing_from_catalog(&plugins).unwrap();
    let previous = super::super::previous_measurements::read(&path);
    let (reused, unmeasured) =
        super::super::previous_measurements::partition(&listing.pairs, previous);
    assert_eq!(listing.pairs.len(), expected);
    assert!(
        unmeasured.is_empty(),
        "isolated rebuild should reuse all load results; first missing: {:?}",
        unmeasured.first(),
    );
    assert_eq!(reused.len(), expected);
    let started = Instant::now();
    let summary = super::super::build_catalog(
        &cmrt_runtime::Config::default(),
        path.clone(),
        path.with_file_name("sources.json"),
        plugins,
        &[],
    )
    .unwrap();
    println!(
        "全{expected}件の再構築: {:.3}秒",
        started.elapsed().as_secs_f64()
    );
    assert_eq!(summary.patch_count, expected);
    assert_eq!(summary.reused_load_count, expected);
    assert_eq!(
        Cache::load_next_to(&path).unwrap().unwrap().records.len(),
        expected
    );
}

#[test]
#[ignore = "requires CMRT_CC1_TEST_CATALOG copied from the installed catalog, before or after rebuild"]
fn restore_full_catalog() {
    let path = fixture_path();
    let sidecar = Cache::load_next_to(&path).unwrap();
    let started = Instant::now();
    let (snapshot, _) = super::super::load_from(&path).unwrap().into_parts();
    let total = snapshot.audio_patches().len();
    assert!(total > 12);
    if let Some(sidecar) = &sidecar {
        assert_eq!(sidecar.records.len(), total);
    }
    for patch in snapshot.audio_patches() {
        let restored = snapshot.cc1_assessment(&patch.reference);
        assert_eq!(
            restored.is_some(),
            sidecar.is_some(),
            "{:?}",
            patch.reference
        );
        if let Some(restored) = restored {
            assert_eq!(restored.patch, patch.reference);
            assert_eq!(
                snapshot
                    .cc1_pitch_change_patches()
                    .contains(&patch.reference.display),
                restored.pitch_change_likely,
                "{:?}",
                patch.reference
            );
        }
    }
    let mut mismatched = snapshot.audio_patches()[0].reference.clone();
    mismatched.plugin = cmrt_core::PluginKey::from_identity(Some("different.plugin"), "");
    assert!(snapshot.cc1_assessment(&mismatched).is_none());
    println!(
        "別プロセスで{total}件を復元 CC1={} ({:.3}秒)",
        sidecar.is_some(),
        started.elapsed().as_secs_f64()
    );
}

fn fixture_path() -> PathBuf {
    let path = PathBuf::from(std::env::var_os("CMRT_CC1_TEST_CATALOG").unwrap());
    let canonical = fs::canonicalize(&path).unwrap();
    if let Some(live) = cmrt_runtime::config_app_dir()
        .and_then(|dir| fs::canonicalize(dir.join("patch-catalog")).ok())
    {
        assert_ne!(
            canonical.parent(),
            Some(live.as_path()),
            "use a copy of the real catalog"
        );
    }
    canonical
}
