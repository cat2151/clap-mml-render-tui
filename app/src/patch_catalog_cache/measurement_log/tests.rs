use super::*;

fn temp_path(label: &str) -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cmrt_measurement_log_{label}_{}_{}.jsonl",
        std::process::id(),
        suffix
    ))
}

fn measured(ms: u64) -> PatchLoadMeasurement {
    PatchLoadMeasurement {
        second_load_ms: Some(ms),
        ..PatchLoadMeasurement::default()
    }
}

#[test]
fn appended_entries_survive_reopening_the_log() {
    let path = temp_path("reopen");
    Writer::open(&path)
        .unwrap()
        .append("Dexed/A", &measured(9))
        .unwrap();
    Writer::open(&path)
        .unwrap()
        .append("Dexed/B", &measured(12))
        .unwrap();

    let entries = read(&path);
    let _ = fs::remove_file(&path);

    assert_eq!(entries["Dexed/A"], measured(9));
    assert_eq!(entries["Dexed/B"], measured(12));
}

#[test]
fn a_half_written_last_line_is_ignored() {
    let path = temp_path("torn");
    Writer::open(&path)
        .unwrap()
        .append("Dexed/A", &measured(9))
        .unwrap();
    OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(br#"{"display":"Dexed/B","second_lo"#)
        .unwrap();

    let entries = read(&path);
    let _ = fs::remove_file(&path);

    assert_eq!(entries.keys().collect::<Vec<_>>(), vec!["Dexed/A"]);
}

#[test]
fn missing_log_reads_as_empty() {
    assert!(read(&temp_path("missing")).is_empty());
}
