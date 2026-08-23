use super::*;

/// entryを渡さない経路（render server backend / テスト）は利用不可のまま。
#[test]
fn none_is_not_available() {
    let entries = PluginEntries::none();
    let key = cmrt_core::PluginKey::from_identity(Some("test.plugin"), "");

    assert!(!entries.is_available());
    assert!(entries.entry(&key).is_err());
}

/// カタログに載っていないkeyを引いても落ちず、「そのpluginは鳴らせない」を返す。
#[test]
fn unknown_key_reports_no_entry() {
    let entries = PluginEntries::none();
    let key = cmrt_core::PluginKey::from_identity(Some("missing.plugin"), "");

    assert!(entries.entry(&key).is_err());
}

#[test]
fn pending_entries_report_loading_then_the_published_error() {
    let entries = PluginEntries::pending();
    assert!(entries
        .loaded()
        .err()
        .unwrap()
        .to_string()
        .contains("準備中"));

    entries.publish_error("cache unavailable");

    assert_eq!(
        entries.loaded().err().unwrap().to_string(),
        "cache unavailable"
    );
}
