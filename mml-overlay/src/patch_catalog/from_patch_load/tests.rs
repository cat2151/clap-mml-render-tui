use super::*;

#[test]
fn a_loading_state_becomes_a_loading_catalog() {
    let host = host_patch_catalog(&PatchLoadState::Loading);
    assert_eq!(host.catalog, PatchCatalogSnapshot::Loading);
    assert!(host.load_measurements.is_empty());
}

#[test]
fn an_error_state_carries_its_reason() {
    let host = host_patch_catalog(&PatchLoadState::Err("no patches_dirs".to_string()));
    assert_eq!(
        host.catalog,
        PatchCatalogSnapshot::Error("no patches_dirs".to_string())
    );
}

#[test]
fn a_ready_state_becomes_selector_rows_in_catalog_order() {
    let state = PatchLoadState::ready(vec![
        ("Bass/Deep.fxp".to_string(), "bass/deep.fxp".to_string()),
        ("Lead/Bright.fxp".to_string(), "lead/bright.fxp".to_string()),
    ]);
    let host = host_patch_catalog(&state);
    let PatchCatalogSnapshot::Ready(entries) = host.catalog else {
        panic!("ready state should produce a ready catalog");
    };
    assert_eq!(
        entries
            .iter()
            .map(PatchCatalogEntry::display)
            .collect::<Vec<_>>(),
        vec!["Bass/Deep.fxp", "Lead/Bright.fxp"]
    );
}
