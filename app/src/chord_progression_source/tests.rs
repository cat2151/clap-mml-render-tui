use std::{fs, path::PathBuf};

use super::*;

const CATALOG: &str = r#"[{"degrees":"I-IV-V-I","description":"test"}]"#;

fn unique_temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cmrt-chord-progression-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn spawn_for_test(config_dir: &std::path::Path, source: &str) -> ChordProgressionSource {
    let refresh = ChordProgressionSource {
        source: Some(CachedSource::new(
            "chord-progressions",
            config_dir,
            source.to_string(),
            "chord-progressions/progressions.json",
        )),
        completion: Arc::new((Mutex::new(false), Condvar::new())),
        io_lock: Arc::new(Mutex::new(())),
        updated: Arc::new(AtomicBool::new(false)),
    };
    refresh.spawn_worker();
    refresh
}

#[test]
fn catalog_json_validation_matches_the_parser() {
    assert!(validate_catalog_json(CATALOG.as_bytes()).is_ok());
    assert!(validate_catalog_json(b"[]").is_err());
    assert!(validate_catalog_json(b"not json").is_err());
}

#[test]
fn first_load_waits_for_the_missing_cache() {
    let temp = unique_temp_dir("first-load");
    fs::create_dir_all(&temp).unwrap();
    fs::write(temp.join("source.json"), CATALOG).unwrap();
    let source = spawn_for_test(&temp, "source.json");

    let catalog = source.catalog();

    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog.entries()[0].degrees, "I-IV-V-I");
    assert!(
        !source.take_update_notice(),
        "初回取得は「更新」として通知しない"
    );
    fs::remove_dir_all(temp).ok();
}

#[test]
fn unavailable_source_yields_an_empty_catalog() {
    let temp = unique_temp_dir("unavailable");
    let source = spawn_for_test(&temp, "missing.json");

    assert!(source.catalog().is_empty());
    assert!(!source.take_update_notice());
    fs::remove_dir_all(temp).ok();
}

#[test]
fn changed_cache_reports_the_update_once() {
    let temp = unique_temp_dir("updated");
    fs::create_dir_all(temp.join("chord-progressions")).unwrap();
    fs::write(
        temp.join("chord-progressions/progressions.json"),
        r#"[{"degrees":"I-V","description":"old"}]"#,
    )
    .unwrap();
    fs::write(temp.join("source.json"), CATALOG).unwrap();
    let source = spawn_for_test(&temp, "source.json");
    // キャッシュが既にあるので catalog() は待たない。取得結果を見たいので明示的に待つ。
    source.wait_for_worker();

    let catalog = source.catalog();

    assert_eq!(catalog.entries()[0].degrees, "I-IV-V-I");
    assert!(source.take_update_notice(), "内容が変わったら一度だけ通知");
    assert!(!source.take_update_notice(), "2回目は通知しない");
    fs::remove_dir_all(temp).ok();
}

#[test]
fn disabled_source_never_blocks_or_notifies() {
    let source = ChordProgressionSource::disabled();
    assert!(source.catalog().is_empty());
    assert!(!source.take_update_notice());
}
