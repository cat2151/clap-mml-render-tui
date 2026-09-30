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
        })
        .unwrap();
        assert_eq!(load_settings().startup_instrument, StartupInstrument::Full);
    });
}
