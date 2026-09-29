use super::*;

fn presets(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(role, pattern)| (role.to_string(), pattern.to_string()))
        .collect()
}

#[test]
fn new_snapshot_starts_with_empty_role_presets() {
    let snapshot = PatchCatalogSnapshot::from_pairs(vec![(
        "Basses/Sub.fxp".to_string(),
        "basses/sub".to_string(),
    )]);
    assert!(snapshot.role_presets().is_empty());
}

#[test]
fn rebuild_patch_roles_keeps_the_user_presets_it_was_given() {
    let mut snapshot = PatchCatalogSnapshot::from_pairs(vec![(
        "Basses/Sub.fxp".to_string(),
        "basses/sub".to_string(),
    )]);
    let user_presets = presets(&[("bass", "sub"), ("lead", "sync")]);

    snapshot.rebuild_patch_roles(&user_presets);
    assert_eq!(snapshot.role_presets(), user_presets.as_slice());

    snapshot.rebuild_patch_roles(&[]);
    assert!(snapshot.role_presets().is_empty());
}

#[test]
fn heavy_offline_load_starts_at_the_byte_threshold_and_ignores_unknown_weight() {
    let with_bytes = |bytes| PatchLoadMeasurement {
        sfz_sample_bytes: bytes,
        ..PatchLoadMeasurement::default()
    };
    assert!(!with_bytes(None).is_heavy_offline_load());
    assert!(!with_bytes(Some(HEAVY_OFFLINE_LOAD_BYTES - 1)).is_heavy_offline_load());
    assert!(with_bytes(Some(HEAVY_OFFLINE_LOAD_BYTES)).is_heavy_offline_load());
}
