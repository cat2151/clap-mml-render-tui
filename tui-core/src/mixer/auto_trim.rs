//! レンダリング済みサンプルから mixer の初期音量（dB）を決める auto trim。
//!
//! Grid Sequencer から Daily DAW へ import する曲の mixer 初期値は、これまで全 track
//! 一律 0dB だった。track ごとの音量差がそのまま残り、大きすぎる track が混ざる。
//!
//! ここでは先頭 1 小節を offline render した結果から track ごとの RMS を測り、
//! **全 track を 0dB 以下へ収める**方向で初期値を決める。上へ揃えないのは
//! [`super::MIXER_MAX_DB`] が +6dB しかないのに対し [`super::MIXER_MIN_DB`] は
//! -36dB あるという非対称性と、mix が単純加算でクリップ対策を持たないためである。

use super::{MIXER_MIN_DB, MIXER_STEP_DB};

/// 全 track を mix したときに狙う RMS。track 数に応じて 1 track ぶんを下げる。
/// play server 側の live auto gain（`MIX_TARGET_RMS_DB`）と同じ思想・同じ値。
const MIX_TARGET_RMS_DB: f32 = -12.0;
/// これを下回る track は「測れなかった」扱いにして 0dB のまま据え置く。
const SILENCE_GATE_DB: f32 = -60.0;
/// 1 track ぶんの補正量の下限・上限。オフセットを掛ける前にここでクランプすることで、
/// 極端に小さい track が 1 本混ざっても全体が沈み込まないようにする。
const TRIM_MIN_DB: f32 = -12.0;
const TRIM_MAX_DB: f32 = 6.0;
/// 補正後の peak がこれを超える track はさらに下げる。RMS だけで揃えると、
/// drum のようなトランジェント主体の track が持ち上がって痛くなるため。
const PEAK_CEILING_DB: f32 = -1.0;

/// 1 track ぶんの測定結果。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackLevel {
    pub track: usize,
    pub rms_db: f32,
    pub peak_db: f32,
}

/// レンダリング済みサンプル列の RMS / peak を測る。
///
/// `samples` は interleaved stereo でも mono でもよい（全サンプルの二乗平均を取るだけ）。
/// 無音ゲート（[`SILENCE_GATE_DB`]）を下回るか、有限値でない場合は `None` を返す。
pub fn measure_track_level(track: usize, samples: &[f32]) -> Option<TrackLevel> {
    if samples.is_empty() {
        return None;
    }
    let sum = samples.iter().map(|sample| sample * sample).sum::<f32>();
    let power = sum / samples.len() as f32;
    if !power.is_finite() || power <= 0.0 {
        return None;
    }
    let rms_db = 10.0 * power.log10();
    if rms_db < SILENCE_GATE_DB {
        return None;
    }
    let peak = samples
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    if !peak.is_finite() || peak <= 0.0 {
        return None;
    }
    Some(TrackLevel {
        track,
        rms_db,
        peak_db: 20.0 * peak.log10(),
    })
}

/// 測定済み track 群から mixer の初期音量（dB）を決める。
///
/// 返り値は `track_count` 長。`levels` に無い track（無音・未測定）は 0dB のまま。
///
/// 手順:
/// 1. 目標 RMS を track 数で割った値に対する各 track の必要補正量を求め、
///    [`TRIM_MIN_DB`]..=[`TRIM_MAX_DB`] へクランプする
/// 2. 最大の補正量が 0dB になるオフセットを全 track へ加える
///    （＝ 一番小さかった track が 0dB、他はすべてマイナス）
/// 3. [`MIXER_STEP_DB`] 刻みへ丸め、peak が [`PEAK_CEILING_DB`] を超える track はさらに下げる
pub fn auto_trim_volumes_db(levels: &[TrackLevel], track_count: usize) -> Vec<i32> {
    let mut volumes_db = vec![0; track_count];
    if levels.is_empty() {
        return volumes_db;
    }

    let target_rms_db = MIX_TARGET_RMS_DB - 10.0 * (levels.len() as f32).log10();
    let trims_db: Vec<f32> = levels
        .iter()
        .map(|level| (target_rms_db - level.rms_db).clamp(TRIM_MIN_DB, TRIM_MAX_DB))
        .collect();
    // 上へ揃えられないぶんを全体で下へ吸収する。相対バランスはオフセットで変わらない。
    let offset_db = -trims_db.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    for (level, trim_db) in levels.iter().zip(trims_db) {
        let Some(slot) = volumes_db.get_mut(level.track) else {
            continue;
        };
        *slot = clamp_below_peak_ceiling(round_to_step(trim_db + offset_db), level.peak_db);
    }
    volumes_db
}

/// mixer が扱う [`MIXER_STEP_DB`] 刻みの整数 dB へ丸める。
/// ユーザーがあとから `+`/`-` したときに同じ格子へ乗るようにするため。
fn round_to_step(volume_db: f32) -> i32 {
    let step = MIXER_STEP_DB as f32;
    let rounded = (volume_db / step).round() as i32 * MIXER_STEP_DB;
    rounded.clamp(MIXER_MIN_DB, 0)
}

/// 補正後の peak が天井を超えるあいだ 1 step ずつ下げる。
fn clamp_below_peak_ceiling(mut volume_db: i32, peak_db: f32) -> i32 {
    while volume_db > MIXER_MIN_DB && peak_db + volume_db as f32 > PEAK_CEILING_DB {
        volume_db -= MIXER_STEP_DB;
    }
    volume_db.max(MIXER_MIN_DB)
}

#[cfg(test)]
mod tests;
