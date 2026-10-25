use std::collections::BTreeSet;

use super::*;

fn tracking(table: &PitchTrackingTable) -> BTreeSet<(String, String)> {
    table
        .tracking()
        .map(|(plugin_id, display)| (plugin_id.to_string(), display.to_string()))
        .collect()
}

/// アプリの取得経路（remote、取れなければ cmrt の cache）が、兄弟 repo の表と同じ中身を返す。
/// push 前の表は remote に無いので、cache へ copy してから走らせる。
#[test]
#[ignore = "ネットワークと cmrt の設定ディレクトリの cache、../cat-music-patterns の表が要る"]
fn installed_table_loads_the_same_entries_as_the_sibling_file() {
    let sibling = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../cat-music-patterns")
        .join(PITCH_TRACKING_FILE);
    let expected = PitchTrackingTable::parse(&std::fs::read(sibling).unwrap()).unwrap();
    let config_dir = cmrt_runtime::config_app_dir().unwrap();
    let loaded = load_in(&config_dir).unwrap();
    println!("tracking entries: {}", tracking(&loaded).len());
    assert_eq!(tracking(&loaded), tracking(&expected));
}
