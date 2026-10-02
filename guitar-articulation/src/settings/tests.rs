use super::*;

fn with_temp_settings<T>(name: &str, body: impl FnOnce() -> T) -> T {
    let tmp = cmrt_history::test_support::unique_test_dir(&format!("guitar_articulation_{name}"));
    let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
    let result = body();
    std::fs::remove_dir_all(&tmp).ok();
    result
}

#[test]
fn missing_file_starts_with_lite_then_full() {
    with_temp_settings("settings_missing", || {
        assert_eq!(
            load_settings().startup_instrument,
            StartupInstrument::LiteThenFull
        );
    });
}

#[test]
fn save_then_load_keeps_startup_instrument() {
    with_temp_settings("settings_roundtrip", || {
        save_settings(&GuitarArticulationSettings {
            startup_instrument: StartupInstrument::Full,
            ..GuitarArticulationSettings::default()
        })
        .unwrap();
        assert_eq!(load_settings().startup_instrument, StartupInstrument::Full);
    });
}

#[test]
fn a_file_without_materials_reads_an_empty_list() {
    let settings: GuitarArticulationSettings =
        serde_json::from_str(r#"{"startup_instrument": "full"}"#).unwrap();
    assert_eq!(settings.startup_instrument, StartupInstrument::Full);
    assert!(settings.arp_materials.is_empty());
}

#[test]
fn save_then_load_keeps_the_materials() {
    with_temp_settings("settings_materials", || {
        let materials = vec!["C".to_string(), "l16cdefgab<c".to_string()];
        save_settings(&GuitarArticulationSettings {
            arp_materials: materials.clone(),
            ..GuitarArticulationSettings::default()
        })
        .unwrap();
        assert_eq!(load_settings().arp_materials, materials);
    });
}

#[test]
fn a_file_without_the_overlay_state_reads_no_material_and_the_default_arp() {
    let settings: GuitarArticulationSettings =
        serde_json::from_str(r#"{"startup_instrument": "full", "arp_materials": ["C"]}"#).unwrap();
    assert_eq!(settings.arp_material, "");
    assert_eq!(settings.arp, ArpSettings::default());
}

#[test]
fn save_then_load_keeps_the_overlay_material_and_arp() {
    with_temp_settings("settings_overlay", || {
        let arp = ArpSettings {
            pattern: cmrt_arpeggiator::ArpPattern::DownUp,
            octaves: 2,
            down: Some(3),
            shift: -1,
            bpm: 90,
            ..ArpSettings::default()
        };
        save_settings(&GuitarArticulationSettings {
            arp_material: "Am7".to_string(),
            arp,
            ..GuitarArticulationSettings::default()
        })
        .unwrap();
        let loaded = load_settings();
        assert_eq!(loaded.arp_material, "Am7");
        assert_eq!(loaded.arp, arp);
    });
}
