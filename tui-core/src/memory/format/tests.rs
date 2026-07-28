use super::*;

const MIB: u64 = 1024 * 1024;

fn process(name: &str, working_set_bytes: u64) -> ProcessMemory {
    ProcessMemory {
        name: name.to_string(),
        pid: 1234,
        working_set_bytes,
    }
}

fn ready(processes: Vec<ProcessMemory>) -> MemoryReading {
    let total_working_set_bytes = processes
        .iter()
        .map(|process| process.working_set_bytes)
        .sum();

    MemoryReading::Ready(MemorySnapshot {
        processes,
        total_working_set_bytes,
        os_available_bytes: 9 * 1024 * MIB,
    })
}

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
fn the_total_line_width_is_the_same_for_every_reading() {
    let widths = [
        MemoryReading::Measuring,
        MemoryReading::Unavailable,
        ready(vec![process("cmrt.exe", 812 * MIB)]),
        ready(vec![process("cmrt.exe", 1023 * 1024 * MIB)]),
    ]
    .map(|reading| overlay_lines(reading)[0].width());

    assert!(widths.iter().all(|width| *width == widths[0]), "{widths:?}");
}

/// 内訳行も同じ理由で、名前の長さ・値の桁数・プロセス数によらず一定幅であること。
#[test]
fn the_detail_line_width_does_not_depend_on_the_contents() {
    let readings = [
        ready(vec![process("cmrt.exe", 0)]),
        ready(vec![process(
            "clap-mml-realtime-play-server.exe",
            1023 * 1024 * MIB,
        )]),
        ready(vec![
            process("cmrt.exe", 62 * MIB),
            process("clap-mml-realtime-play-server.exe", 438 * MIB),
            process("clap-mml-render-server.exe", 12 * MIB),
        ]),
    ];

    let widths: Vec<usize> = readings
        .into_iter()
        .flat_map(|reading| {
            let lines = overlay_lines(reading);
            // 先頭の合計行と末尾の空行を除いた内訳行だけを見る。
            lines[1..lines.len() - 1]
                .iter()
                .map(Line::width)
                .collect::<Vec<_>>()
        })
        .collect();

    assert!(widths.iter().all(|width| *width == widths[0]), "{widths:?}");
}

/// 計測が済むまでは内訳を出さない。help の高さが計測状態で跳ねないようにするため。
#[test]
fn a_reading_without_a_snapshot_stays_two_lines() {
    for reading in [MemoryReading::Measuring, MemoryReading::Unavailable] {
        let lines = overlay_lines(reading);

        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[1].width(), 0);
    }
}

#[test]
fn a_ready_reading_shows_both_values() {
    let lines = overlay_lines(MemoryReading::Ready(MemorySnapshot {
        processes: Vec::new(),
        total_working_set_bytes: 1208 * MIB,
        os_available_bytes: 812 * MIB,
    }));
    let text = lines[0].to_string();

    assert!(text.contains("実メモリ合計"), "{text}");
    assert!(text.contains("1.18 GB"), "{text}");
    assert!(text.contains("OS空き"), "{text}");
    assert!(text.contains("812 MB"), "{text}");
}

#[test]
fn a_ready_reading_lists_every_process_between_the_total_and_the_blank_line() {
    let lines = overlay_lines(ready(vec![
        process("cmrt.exe", 62 * MIB),
        process("clap-mml-realtime-play-server.exe", 438 * MIB),
    ]));

    assert_eq!(lines.len(), 4, "{lines:?}");
    assert!(lines[1].to_string().contains("cmrt.exe"), "{lines:?}");
    assert!(lines[1].to_string().contains("62 MB"), "{lines:?}");
    assert!(
        lines[2]
            .to_string()
            .contains("clap-mml-realtime-play-server.exe"),
        "{lines:?}"
    );
    assert!(lines[2].to_string().contains("438 MB"), "{lines:?}");
    assert_eq!(lines[3].width(), 0);
}

/// exe をリネームされても枠幅が動かないこと。
#[test]
fn a_name_longer_than_the_column_is_truncated() {
    let lines = overlay_lines(ready(vec![process(&"x".repeat(NAME_WIDTH + 20), MIB)]));
    let text = lines[1].to_string();

    assert!(!text.contains(&"x".repeat(NAME_WIDTH + 1)), "{text}");
    assert!(text.contains(&"x".repeat(NAME_WIDTH)), "{text}");
}

/// 全角名でも桁数で切るので、カラム幅を溢れない。
#[test]
fn a_wide_character_name_is_truncated_by_display_width() {
    assert_eq!(
        Line::from(truncate_to_width("あ".repeat(40).as_str(), 5)).width(),
        4
    );
}

#[test]
fn processes_beyond_the_limit_are_rolled_up_into_one_line() {
    let processes: Vec<ProcessMemory> = (0..MAX_DETAIL_LINES + 3)
        .map(|index| process(&format!("server-{index}.exe"), 10 * MIB))
        .collect();
    let overflowed = processes.len() - (MAX_DETAIL_LINES - 1);

    let lines = overlay_lines(ready(processes));

    // 合計行 + 内訳 MAX_DETAIL_LINES 行 + 空行。
    assert_eq!(lines.len(), MAX_DETAIL_LINES + 2, "{lines:?}");
    let rollup = lines[MAX_DETAIL_LINES].to_string();
    assert!(
        rollup.contains(&format!("他 {overflowed} プロセス")),
        "{rollup}"
    );
    assert!(
        rollup.contains(&format_bytes(overflowed as u64 * 10 * MIB)),
        "{rollup}"
    );
}
