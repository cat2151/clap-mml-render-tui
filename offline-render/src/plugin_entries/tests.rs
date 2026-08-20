use super::*;

/// entry を渡さない経路（render server backend / テスト）は、
/// 「in-process では鳴らせない」を `0` で表し続ける。ここが `true` を返すと
/// 呼び出し側が in-process 経路へ入り、null ポインタを踏む。
#[test]
fn none_is_not_available() {
    let entries = PluginEntries::none();

    assert!(!entries.is_available());
    assert_eq!(entries.ptr(0), 0);
}

/// カタログに載っていない添字を引いても落ちず、「その位置は鳴らせない」を返す。
/// catalog_plugins と entry 列の長さがずれても、静かに別プラグインを掴むより
/// エラーになるほうがよい。
#[test]
fn out_of_range_index_reports_no_entry() {
    let entries = PluginEntries::none();

    assert_eq!(entries.ptr(7), 0);
}
