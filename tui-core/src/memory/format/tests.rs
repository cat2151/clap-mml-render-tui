use super::*;

const MIB: u64 = 1024 * 1024;

#[test]
fn bytes_under_one_gib_are_shown_in_mib() {
    assert_eq!(format_bytes(812 * MIB), "812 MB");
    assert_eq!(format_bytes(0), "0 MB");
}

#[test]
fn bytes_over_one_gib_are_shown_in_gib_with_two_decimals() {
    assert_eq!(format_bytes(1024 * MIB), "1.00 GB");
    assert_eq!(format_bytes(1208 * MIB), "1.18 GB");
}

#[test]
fn bytes_over_one_tib_are_shown_in_tib() {
    assert_eq!(format_bytes(2 * 1024 * 1024 * MIB), "2.00 TB");
}

/// 枠幅が open のたびに伸縮しないことの回帰テスト。
#[test]
fn the_line_width_is_the_same_for_every_reading() {
    let widths = [
        MemoryReading::Measuring,
        MemoryReading::Unavailable,
        MemoryReading::Ready(MemorySnapshot {
            total_working_set_bytes: 812 * MIB,
            os_available_bytes: 9 * 1024 * MIB,
        }),
        MemoryReading::Ready(MemorySnapshot {
            total_working_set_bytes: 1023 * 1024 * MIB,
            os_available_bytes: 0,
        }),
    ]
    .map(|reading| overlay_lines(reading)[0].width());

    assert!(widths.iter().all(|width| *width == widths[0]), "{widths:?}");
}

#[test]
fn overlay_lines_end_with_a_blank_separator() {
    let lines = overlay_lines(MemoryReading::Measuring);

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1].width(), 0);
}

#[test]
fn a_ready_reading_shows_both_values() {
    let lines = overlay_lines(MemoryReading::Ready(MemorySnapshot {
        total_working_set_bytes: 1208 * MIB,
        os_available_bytes: 812 * MIB,
    }));
    let text = lines[0].to_string();

    assert!(text.contains("実メモリ合計"), "{text}");
    assert!(text.contains("1.18 GB"), "{text}");
    assert!(text.contains("OS空き"), "{text}");
    assert!(text.contains("812 MB"), "{text}");
}
