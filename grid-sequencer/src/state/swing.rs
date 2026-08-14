//! instance ごとの shuffle（swing）クオンタイズ。
//!
//! 16分グリッドの裏拍（奇数 step）だけを後ろへずらす。50% は等分＝跳ねなし、
//! 66% はほぼ三連シャッフル（裏の16分が8分音符の 2/3 の位置へ来る）。
//!
//! **「値を持っている」と「効いている」は別**。裏拍に note on が1つも無い instance は
//! ずらす対象が無いので、値を持っていても跳ねない（[`GridState::effective_swings`]）。
//! chord mode の和音行は step 0 で1発鳴らして1meas伸ばすだけなので、この規則だけで
//! 自動的に対象外になる。行の種類を見る専用の分岐は要らない。

use super::{GridState, LaneAddress, CHORD_ROW, GRID_STEPS, STEPS_PER_BEAT};

/// 跳ねなし（等分）。
pub const SWING_MIN: u8 = 50;
/// 標準のシャッフル。裏の16分が8分音符の 2/3 の位置へ来る。
pub const SWING_MAX: u8 = 66;

/// 保存値やランダム値を有効域へ収める。範囲を信じない唯一の窓口。
pub fn clamp_swing(swing: u8) -> u8 {
    swing.clamp(SWING_MIN, SWING_MAX)
}

/// `step` の発音を後ろへずらす秒数。表拍（偶数 step）と 50% は常に 0。
///
/// 秒の絶対値ではなく step 長に対する比率で決める。[`super::clock::StepClock::retempo`]
/// でテンポを乗り換えても跳ね具合が変わらないため。
pub fn swing_offset_seconds(swing: u8, step: usize, bpm: f64) -> f64 {
    if step.is_multiple_of(2) {
        return 0.0;
    }
    let ratio = f64::from(clamp_swing(swing) - SWING_MIN) / f64::from(SWING_MIN);
    ratio * step_seconds(bpm)
}

fn step_seconds(bpm: f64) -> f64 {
    if bpm <= 0.0 {
        return 0.0;
    }
    60.0 / (bpm * STEPS_PER_BEAT as f64)
}

/// 16分の裏拍に note on があるか。表拍の四分・八分だけの行は跳ねようがない。
fn has_offbeat_attack(attacks: &[bool; GRID_STEPS]) -> bool {
    (1..GRID_STEPS).step_by(2).any(|step| attacks[step])
}

impl GridState {
    /// instance ごとの、いま実際に効いている swing。跳ねない行は `None`。
    ///
    /// 保存せずに毎回引き直す。手編集で譜面が変われば、表示も発音も同じ関数を
    /// 通るので勝手に追従する。
    ///
    /// 発音の有無は [`GridState::instance_trigger_table`] に一本化している。drum 行・
    /// chord 行・多声 lane の特例をここで書き直すと、必ずどこかでずれる。
    pub(crate) fn effective_swings(&self) -> Vec<Option<u8>> {
        self.instance_trigger_table()
            .iter()
            .zip(self.instances.iter())
            .map(|(attacks, instance)| {
                has_offbeat_attack(attacks).then(|| clamp_swing(instance.swing))
            })
            .collect()
    }

    /// [`GridState::effective_swings`] の1件版。1行だけ知りたいとき用。
    pub fn effective_swing(&self, instance: usize) -> Option<u8> {
        self.effective_swings().get(instance).copied().flatten()
    }

    /// 描画用スナップショットで実際に効いている swing。先読み側の次 loop を参照して
    /// NOTE grid のセルより先に値だけ切り替わらないよう、発音用とは入口を分ける。
    pub(crate) fn display_effective_swings(&self) -> Vec<Option<u8>> {
        self.display_instances()
            .iter()
            .enumerate()
            .map(|(instance_index, instance)| {
                if self.display_chord().is_some() && instance_index == CHORD_ROW {
                    return None;
                }
                let attacks = std::array::from_fn(|step| {
                    instance.lanes.iter().enumerate().any(|(lane, item)| {
                        item.pattern.is_attack(step)
                            && self
                                .display_resolved_note(LaneAddress::new(instance_index, lane))
                                .is_some()
                    })
                });
                has_offbeat_attack(&attacks).then(|| clamp_swing(instance.swing))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
