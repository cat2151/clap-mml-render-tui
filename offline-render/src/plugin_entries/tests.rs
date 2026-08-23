use super::*;

/// entryを渡さない経路（render server backend / テスト）は利用不可のまま。
#[test]
fn none_is_not_available() {
    let entries = PluginEntries::none();

    assert!(!entries.is_available());
    assert!(entries.entry(0).is_err());
}

/// カタログに載っていない添字を引いても落ちず、「その位置は鳴らせない」を返す。
/// catalog_plugins と entry 列の長さがずれても、静かに別プラグインを掴むより
/// エラーになるほうがよい。
#[test]
fn out_of_range_index_reports_no_entry() {
    let entries = PluginEntries::none();

    assert!(entries.entry(7).is_err());
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
