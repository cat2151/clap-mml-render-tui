use super::DawApp;

#[test]
fn patch_display_path_query_searches_category_vendor_and_patch_name() {
    let patches = vec![
        (
            "patches_factory/Instrument/Soft Strum.fxp".to_string(),
            "patches_factory/instrument/soft strum.fxp".to_string(),
        ),
        (
            "patches_3rdparty/Acme/Guitars/Plain Voice.fxp".to_string(),
            "patches_3rdparty/acme/guitars/plain voice.fxp".to_string(),
        ),
    ];

    for (query, expected) in [
        ("instrument", "patches_factory/Instrument/Soft Strum.fxp"),
        ("acme", "patches_3rdparty/Acme/Guitars/Plain Voice.fxp"),
        ("strum", "patches_factory/Instrument/Soft Strum.fxp"),
    ] {
        assert_eq!(
            DawApp::filter_patch_names_by_display_path_query(&patches, query),
            vec![expected.to_string()]
        );
    }
}
