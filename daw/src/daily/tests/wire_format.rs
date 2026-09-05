//! Daily recovery / Archive の wire 形式・パス・日付分類。

use std::io::{Error, Write};

use super::*;

#[test]
fn daily_paths_share_one_feature_root_and_flat_archive() {
    let config_dir = Path::new("config-root");

    assert_eq!(daily_feature_root(config_dir), config_dir.join("daily_daw"));
    assert_eq!(
        daily_current_path(config_dir),
        config_dir.join("daily_daw").join("current.json")
    );
    assert_eq!(
        daily_archive_root(config_dir),
        config_dir.join("daily_daw").join("archive")
    );
    assert_eq!(
        daily_archive_path(config_dir, "2026-08-26").unwrap(),
        config_dir
            .join("daily_daw")
            .join("archive")
            .join("2026-08-26.cmrt-daw.json")
    );
}

#[test]
fn recovery_wire_roundtrips_existing_project_and_cache_shapes() {
    let recovery = recovery("2026-08-26", "cdef");

    let encoded = serde_json::to_value(&recovery).unwrap();
    let decoded = decode_daily_recovery(&serde_json::to_string(&encoded).unwrap()).unwrap();

    assert_eq!(encoded, recovery_value("2026-08-26", "cdef"));
    assert_eq!(decoded.page_date, "2026-08-26");
    assert_eq!((decoded.cursor_track, decoded.cursor_measure), (1, 2));
    assert_eq!(decoded.cached_measures.len(), 1);
    assert_eq!(decoded.cached_measures[0].mml_hash, 42);
}

#[test]
fn recovery_rejects_invalid_date_and_invalid_nested_project() {
    for date in ["2026-8-26", "2026-02-29", "2026-13-01", "../escape"] {
        let error =
            decode_daily_recovery(&serde_json::to_string(&recovery_value(date, "cdef")).unwrap())
                .unwrap_err()
                .to_string();
        assert!(error.contains("YYYY-MM-DD"), "{date}: {error}");
    }

    let mut invalid_project = recovery_value("2026-08-26", "cdef");
    invalid_project["project_file"]["format"] = json!("not-a-daw-project");
    let error = decode_daily_recovery(&serde_json::to_string(&invalid_project).unwrap())
        .unwrap_err()
        .to_string();
    assert!(error.contains("project が不正です"));
}

#[test]
fn missing_recovery_is_first_use_and_invalid_files_are_errors() {
    let temp = TempDirectory::new("load");
    let current_path = daily_current_path(temp.path());

    assert!(load_daily_recovery(&current_path).unwrap().is_none());

    std::fs::create_dir_all(current_path.parent().unwrap()).unwrap();
    std::fs::write(&current_path, b"not json").unwrap();
    let error = load_daily_recovery(&current_path).unwrap_err().to_string();
    assert!(error.contains("Daily recovery が不正です"));
}

#[test]
fn date_classification_handles_first_use_resume_and_rollover() {
    assert_eq!(
        classify_daily_date(None, "2026-08-26").unwrap(),
        DailyDateClassification::FirstUse
    );
    assert_eq!(
        classify_daily_date(Some("2026-08-26"), "2026-08-26").unwrap(),
        DailyDateClassification::Resume
    );
    assert_eq!(
        classify_daily_date(Some("2026-08-27"), "2026-08-26").unwrap(),
        DailyDateClassification::Resume
    );
    assert_eq!(
        classify_daily_date(Some("2026-08-25"), "2026-08-26").unwrap(),
        DailyDateClassification::Rollover
    );
    assert!(classify_daily_date(Some("2026-02-29"), "2026-08-26").is_err());
    assert!(classify_daily_date(None, "today").is_err());
}

#[test]
fn archive_create_new_never_overwrites_an_existing_snapshot() {
    let temp = TempDirectory::new("already-exists");
    let path = daily_archive_path(temp.path(), "2026-08-26").unwrap();
    let first = recovery("2026-08-26", "cdef");
    let second = recovery("2026-08-26", "gggg");

    assert_eq!(
        write_daily_archive(&path, &first.project_file).unwrap(),
        DailyArchiveOutcome::Created
    );
    let archived = std::fs::read(&path).unwrap();
    assert_eq!(
        write_daily_archive(&path, &second.project_file).unwrap(),
        DailyArchiveOutcome::AlreadyExists
    );

    assert_eq!(std::fs::read(&path).unwrap(), archived);
    let value: Value = serde_json::from_slice(&archived).unwrap();
    assert_eq!(value, project_value("cdef"));
}

#[test]
fn archive_write_failure_removes_partial_file_and_keeps_recovery() {
    let temp = TempDirectory::new("write-failure");
    let recovery = recovery("2026-08-26", "cdef");
    let recovery_before = serde_json::to_vec(&recovery).unwrap();
    let current_path = daily_current_path(temp.path());
    std::fs::create_dir_all(current_path.parent().unwrap()).unwrap();
    std::fs::write(&current_path, &recovery_before).unwrap();
    let path = daily_archive_path(temp.path(), "2026-08-26").unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();

    let error = create_new_archive_with(&path, |file| {
        file.write_all(b"{\"partial\":")?;
        Err(Error::other("injected write failure"))
    })
    .unwrap_err()
    .to_string();

    assert!(error.contains("injected write failure"));
    assert!(!path.exists());
    assert_eq!(serde_json::to_vec(&recovery).unwrap(), recovery_before);
    assert_eq!(std::fs::read(current_path).unwrap(), recovery_before);
}
