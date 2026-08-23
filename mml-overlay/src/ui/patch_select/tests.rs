use super::format_load_time;

#[test]
fn load_time_format_uses_readable_truncated_units() {
    assert_eq!(format_load_time(0), "0ms");
    assert_eq!(format_load_time(20), "20ms");
    assert_eq!(format_load_time(99), "99ms");
    assert_eq!(format_load_time(100), "0.1s");
    assert_eq!(format_load_time(999), "0.9s");
    assert_eq!(format_load_time(1_000), "1s");
    assert_eq!(format_load_time(1_999), "1s");
    assert_eq!(format_load_time(9_000), "9s");
}
