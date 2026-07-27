use super::*;

/// help overlay は待たせずに出したいので、計測は必ずバックグラウンドへ逃がす。
/// `request_refresh` 自体は即座に返り、`overlay_lines` はいつ呼んでもブロックしない。
#[test]
fn request_refresh_returns_immediately_and_overlay_lines_are_always_available() {
    request_refresh();
    request_refresh();

    let lines = overlay_lines();

    assert_eq!(lines.len(), 2);
}

/// `test-support` 有効時（画面クレートの描画テストがこの構成）は実測せず、
/// 表示を「計測中」に固定してテストを決定的にする。
#[cfg(feature = "test-support")]
#[test]
fn the_reading_stays_measuring_under_test_support() {
    request_refresh();

    assert_eq!(probe::reading(), MemoryReading::Measuring);
    assert!(overlay_lines()[0].to_string().contains("計測中"));
}
