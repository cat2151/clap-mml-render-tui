use super::*;

#[test]
fn sort_patch_pairs_can_group_by_category_before_path() {
    let mut pairs = vec![
        (
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_factory/pad/super pad.fxp".to_string(),
        ),
        (
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/john/lead/great lead.fxp".to_string(),
        ),
        (
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
            "patches_3rdparty/john/pad/great pad.fxp".to_string(),
        ),
        (
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/lead/super lead.fxp".to_string(),
        ),
    ];

    sort_patch_pairs(&mut pairs, PatchSortOrder::Category);

    assert_eq!(
        pairs
            .into_iter()
            .map(|(display, _)| display)
            .collect::<Vec<_>>(),
        vec![
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
        ]
    );
}

#[test]
fn sort_patch_pairs_path_order_keeps_factory_before_thirdparty() {
    let mut pairs = vec![
        (
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/john/lead/great lead.fxp".to_string(),
        ),
        (
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_factory/pad/super pad.fxp".to_string(),
        ),
        (
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
            "patches_3rdparty/john/pad/great pad.fxp".to_string(),
        ),
        (
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/lead/super lead.fxp".to_string(),
        ),
    ];

    sort_patch_pairs(&mut pairs, PatchSortOrder::Path);

    assert_eq!(
        pairs
            .into_iter()
            .map(|(display, _)| display)
            .collect::<Vec<_>>(),
        vec![
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
        ]
    );
}

#[test]
fn sort_patch_pairs_category_order_handles_vendorless_thirdparty_paths() {
    let mut pairs = vec![
        (
            "patches_3rdparty/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/lead/great lead.fxp".to_string(),
        ),
        (
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_factory/pad/super pad.fxp".to_string(),
        ),
        (
            "patches_3rdparty/pad/Great Pad.fxp".to_string(),
            "patches_3rdparty/pad/great pad.fxp".to_string(),
        ),
        (
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/lead/super lead.fxp".to_string(),
        ),
    ];

    sort_patch_pairs(&mut pairs, PatchSortOrder::Category);

    assert_eq!(
        pairs
            .into_iter()
            .map(|(display, _)| display)
            .collect::<Vec<_>>(),
        vec![
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_3rdparty/lead/Great Lead.fxp".to_string(),
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_3rdparty/pad/Great Pad.fxp".to_string(),
        ]
    );
}

#[test]
fn group_patch_pairs_merges_factory_and_thirdparty_categories() {
    let pairs = vec![
        (
            "patches_3rdparty/john/Pad/Pad 2.fxp".to_string(),
            "patches_3rdparty/john/pad/pad 2.fxp".to_string(),
        ),
        (
            "patches_factory/Lead/Lead 1.fxp".to_string(),
            "patches_factory/lead/lead 1.fxp".to_string(),
        ),
        (
            "patches_factory/Pad/Pad 11.fxp".to_string(),
            "patches_factory/pad/pad 11.fxp".to_string(),
        ),
        (
            "patches_factory/Pad/Pad 1.fxp".to_string(),
            "patches_factory/pad/pad 1.fxp".to_string(),
        ),
    ];

    let categories = group_patch_pairs_by_category(&pairs);

    assert_eq!(
        categories,
        vec![
            PatchCategory {
                name: "Lead".to_string(),
                patches: vec!["patches_factory/Lead/Lead 1.fxp".to_string()],
            },
            PatchCategory {
                name: "Pad".to_string(),
                patches: vec![
                    "patches_factory/Pad/Pad 1.fxp".to_string(),
                    "patches_factory/Pad/Pad 11.fxp".to_string(),
                    "patches_3rdparty/john/Pad/Pad 2.fxp".to_string(),
                ],
            },
        ]
    );
}

#[test]
fn group_patch_pairs_handles_vendorless_thirdparty_categories() {
    let pairs = vec![
        (
            "patches_3rdparty/Pad/Third Pad.fxp".to_string(),
            "patches_3rdparty/pad/third pad.fxp".to_string(),
        ),
        (
            "patches_factory/Pad/Factory Pad.fxp".to_string(),
            "patches_factory/pad/factory pad.fxp".to_string(),
        ),
    ];

    let categories = group_patch_pairs_by_category(&pairs);

    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].name, "Pad");
    assert_eq!(
        categories[0].patches,
        vec![
            "patches_factory/Pad/Factory Pad.fxp".to_string(),
            "patches_3rdparty/Pad/Third Pad.fxp".to_string(),
        ]
    );
}

#[test]
fn patch_sort_order_toggles_between_path_and_category() {
    assert_eq!(PatchSortOrder::Path.toggle(), PatchSortOrder::Category);
    assert_eq!(PatchSortOrder::Category.toggle(), PatchSortOrder::Path);
    assert_eq!(PatchSortOrder::Path.status_label(), "path");
    assert_eq!(PatchSortOrder::Category.status_label(), "category");
}
