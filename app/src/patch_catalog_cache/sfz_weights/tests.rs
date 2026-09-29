use super::*;
use std::fs;
use std::path::PathBuf;

fn temp_root(label: &str) -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir()
        .join(format!(
            "cmrt_sfz_weights_{label}_{}_{suffix}",
            std::process::id()
        ))
        .join("Library")
}

fn sforzando(root: &Path) -> CatalogPlugin {
    let roots = vec![root.to_string_lossy().into_owned()];
    CatalogPlugin {
        name: "Sforzando".to_string(),
        plugin_path: "C:/sforzando.clap".to_string(),
        plugin_id: Some(cmrt_runtime::SFORZANDO_PLUGIN_ID.to_string()),
        base: cmrt_runtime::PatchBase::per_root(&roots),
        dirs: roots,
        resolved_patches: None,
        source_notices: Vec::new(),
    }
}

fn weighed(bytes: u64, files: u32) -> PatchLoadMeasurement {
    PatchLoadMeasurement {
        sfz_sample_files: Some(files),
        sfz_sample_bytes: Some(bytes),
        ..PatchLoadMeasurement::default()
    }
}

#[test]
fn record_resolves_the_display_through_the_plugin_base_and_overwrites_old_values() {
    let root = temp_root("record");
    fs::create_dir_all(root.join("Piano/samples")).unwrap();
    fs::write(root.join("Piano/samples/a.wav"), [0u8; 10]).unwrap();
    fs::write(
        root.join("Piano/Piano.sfz"),
        "<region> sample=samples/a.wav\n<region> sample=samples/missing.wav\n",
    )
    .unwrap();
    let mut measurements = BTreeMap::from([
        ("Library/Piano/Piano.sfz".to_string(), weighed(1, 1)),
        ("Library/Piano/Gone.sfz".to_string(), weighed(1, 1)),
        ("Library/Piano/Preset.ariax".to_string(), weighed(1, 1)),
    ]);

    let failed = record(&[sforzando(&root)], &mut measurements);

    assert_eq!(
        measurements["Library/Piano/Piano.sfz"],
        weighed(10, 2),
        "欠損も数え、存在する sample だけ容量へ足す"
    );
    assert_eq!(
        measurements["Library/Piano/Gone.sfz"],
        PatchLoadMeasurement::default()
    );
    assert_eq!(
        measurements["Library/Piano/Preset.ariax"],
        PatchLoadMeasurement::default()
    );
    assert_eq!(failed, vec!["Library/Piano/Gone.sfz".to_string()]);
    let _ = fs::remove_dir_all(root.parent().unwrap());
}

#[test]
fn distribution_lines_show_percentiles_heavy_count_and_heaviest_first() {
    let measurements = BTreeMap::from([
        ("a.sfz".to_string(), weighed(1_000_000, 3)),
        ("b.sfz".to_string(), weighed(HEAVY_OFFLINE_LOAD_BYTES, 300)),
        ("c.sfz".to_string(), weighed(2_000_000, 4)),
        ("d.fxp".to_string(), PatchLoadMeasurement::default()),
    ]);

    let lines = distribution_lines(&measurements, &["x.sfz".to_string()]);

    assert_eq!(
        lines,
        vec![
            "sfz sample集計: sfz=3 集計失敗=1 p50=2.0MB p90=64.0MB p99=64.0MB 重い(>=64.0MB)=1",
            "  集計失敗: x.sfz",
            "sfz sample総容量の上位20件:",
            "    300 files     64.0MB  b.sfz",
            "      4 files      2.0MB  c.sfz",
            "      3 files      1.0MB  a.sfz",
        ]
    );
}
