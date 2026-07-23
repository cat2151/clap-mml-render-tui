//! mixer の音量（dB）ドメイン。
//!
//! DAW ミキサーと loop browser の両画面から共有される、UI に依存しない値ロジック。

#[cfg(test)]
mod tests;

pub const MIXER_MIN_DB: i32 = -36;
pub const MIXER_MAX_DB: i32 = 6;
pub const MIXER_STEP_DB: i32 = 3;

/// `volume_db` を `delta_db` だけ増減し、共有境界 `[MIXER_MIN_DB, MIXER_MAX_DB]` に丸める。
/// 値が実際に変化した場合のみ `true` を返す。
pub fn adjust_volume_db(volume_db: &mut i32, delta_db: i32) -> bool {
    let next = (*volume_db + delta_db).clamp(MIXER_MIN_DB, MIXER_MAX_DB);
    if next == *volume_db {
        return false;
    }
    *volume_db = next;
    true
}

/// dB 値を線形ゲインへ変換する。
pub fn volume_db_to_gain(volume_db: i32) -> f32 {
    10.0f32.powf(volume_db as f32 / 20.0)
}
