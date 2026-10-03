use std::sync::Arc;

use cmrt_patch_select::{host_patch_catalog, PatchAuditionContext, PatchAuditionSelect};
use cmrt_patches::PatchRole;
use cmrt_tui_core::patch_load::PatchLoadState;

use super::*;

/// 実 catalog cache で、`patch` を Chord Chart と同じく Chord role 指定で開いたとき
/// カーソルが `patch` を指すかを確かめる。role と開いた先の preset を標準エラーへ出す。
///
/// `CMRT_TEST_CATALOG_CACHE` に catalog cache の path、`CMRT_TEST_CHORD_PATCH` に音色の表示名を渡す。
#[test]
#[ignore = "real catalog cache is required: set CMRT_TEST_CATALOG_CACHE and CMRT_TEST_CHORD_PATCH"]
fn installed_catalog_chord_role_open_points_at_the_patch() {
    let (Ok(cache), Ok(patch)) = (
        std::env::var("CMRT_TEST_CATALOG_CACHE"),
        std::env::var("CMRT_TEST_CHORD_PATCH"),
    ) else {
        eprintln!("skip: CMRT_TEST_CATALOG_CACHE / CMRT_TEST_CHORD_PATCH is not set");
        return;
    };
    let (mut snapshot, _) = load_from(Path::new(&cache)).unwrap().into_parts();
    snapshot.rebuild_patch_roles(&crate::history::load_mml_patch_filter_presets());
    let state = PatchLoadState::Ready(Arc::new(snapshot));
    let host = host_patch_catalog(&state);
    eprintln!(
        "role_of={:?} drum_role_of={:?}",
        host.patch_role_index.role_of(&patch),
        host.patch_role_index.drum_role_of(&patch)
    );

    let mut opened = Vec::new();
    for initial_role in [Some(PatchRole::Chord), None] {
        let mut select = PatchAuditionSelect::default();
        select.open(PatchAuditionContext {
            catalog: host.catalog.clone(),
            patch_role_index: host.patch_role_index.clone(),
            initial_role,
            load_measurements: host.load_measurements.clone(),
            filter_presets: crate::history::load_mml_patch_filter_presets(),
            ..Default::default()
        });
        select.set_patch(Some(patch.clone()));
        select.request_select();
        let patch_select = select
            .select()
            .expect("catalog cache should open the selector");
        let preset = &patch_select.presets()[patch_select.preset_cursor()];
        eprintln!(
            "initial_role={initial_role:?} group={} preset={:?} cursor={} selected={:?}",
            preset.group.label(),
            preset.label,
            patch_select.cursor(),
            patch_select.selected()
        );
        opened.push(patch_select.selected().map(str::to_string));
    }
    assert_eq!(opened[0].as_deref(), Some(patch.as_str()));
}
