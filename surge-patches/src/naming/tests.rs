use super::*;

#[test]
fn normalize_patch_lookup_key_unifies_separators_and_case() {
    assert_eq!(
        normalize_patch_lookup_key(r" .\patches_factory\Pads\Warm Pad.fxp "),
        "patches_factory/pads/warm pad.fxp"
    );
}

#[test]
fn compare_patch_names_natural_orders_numeric_suffixes() {
    let mut items = vec![
        "Pads/Pad 11.fxp".to_string(),
        "Pads/Pad 2.fxp".to_string(),
        "Pads/Pad 1.fxp".to_string(),
    ];
    items.sort_by(|left, right| compare_patch_names_natural(left, right));

    assert_eq!(
        items,
        vec![
            "Pads/Pad 1.fxp".to_string(),
            "Pads/Pad 2.fxp".to_string(),
            "Pads/Pad 11.fxp".to_string(),
        ]
    );
}

#[test]
fn compare_normalized_patch_names_natural_orders_numeric_suffixes() {
    let mut items = vec![
        "pads/pad 11.fxp".to_string(),
        "pads/pad 2.fxp".to_string(),
        "pads/pad 1.fxp".to_string(),
    ];
    items.sort_by(|left, right| compare_normalized_patch_names_natural(left, right));

    assert_eq!(
        items,
        vec![
            "pads/pad 1.fxp".to_string(),
            "pads/pad 2.fxp".to_string(),
            "pads/pad 11.fxp".to_string(),
        ]
    );
}

#[test]
fn resolve_display_patch_name_adds_factory_prefix_when_missing() {
    let pairs = vec![
        (
            "patches_factory/Pads/Factory Pad.fxp".to_string(),
            "patches_factory/pads/factory pad.fxp".to_string(),
        ),
        (
            "patches_3rdparty/Leads/Third Lead.fxp".to_string(),
            "patches_3rdparty/leads/third lead.fxp".to_string(),
        ),
    ];

    let resolved = resolve_display_patch_name(&pairs, "Pads/Factory Pad.fxp");

    assert_eq!(
        resolved.as_deref(),
        Some("patches_factory/Pads/Factory Pad.fxp")
    );
}

#[test]
fn resolve_display_patch_name_prefers_existing_prefixed_name() {
    let pairs = vec![(
        "patches_3rdparty/Leads/Third Lead.fxp".to_string(),
        "patches_3rdparty/leads/third lead.fxp".to_string(),
    )];

    let resolved = resolve_display_patch_name(&pairs, "patches_3rdparty/Leads/Third Lead.fxp");

    assert_eq!(
        resolved.as_deref(),
        Some("patches_3rdparty/Leads/Third Lead.fxp")
    );
}

#[test]
fn resolve_display_patch_name_returns_none_for_an_empty_name() {
    let pairs = vec![(
        "patches_factory/Pads/Factory Pad.fxp".to_string(),
        "patches_factory/pads/factory pad.fxp".to_string(),
    )];

    assert_eq!(resolve_display_patch_name(&pairs, "   "), None);
}
