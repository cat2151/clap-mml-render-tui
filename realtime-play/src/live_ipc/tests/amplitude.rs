//! dB とリニア振幅の変換。SHM が dB を u32 の千分率で運ぶので、丸めのずれも見る。

use crate::live_ipc::amplitude_from_db;

#[test]
fn zero_db_is_unity_gain() {
    assert!((amplitude_from_db(0.0) - 1.0).abs() < 1e-6);
}

#[test]
fn plus_six_db_roughly_doubles_the_amplitude() {
    let gain = amplitude_from_db(6.0);
    assert!((gain - 1.9953).abs() < 1e-3, "gain={gain}");
}

#[test]
fn minus_six_db_roughly_halves_the_amplitude() {
    let gain = amplitude_from_db(-6.0);
    assert!((gain - 0.5012).abs() < 1e-3, "gain={gain}");
}

/// 千分率へ丸めても dB のずれが無視できること（SHM が u32 milli で運ぶため）。
#[test]
fn rounding_to_milli_units_keeps_the_db_within_a_hundredth() {
    let gain = amplitude_from_db(6.0);
    let rounded = (gain * 1000.0).round() / 1000.0;
    let db = 20.0 * rounded.log10();
    assert!((db - 6.0).abs() < 0.01, "db={db}");
}
