//! drum 行の役割の割当と、その譜面の書き込み。
//!
//! どの instance が drum になるかは track 数だけから決まる（[`drum_role_for`]）。
//! index を直に見る判定をこれ以上増やさないため、行番号の知識はこの module へ閉じてある。
//!
//! [`cmrt_rhythm`] は「step 何番で何 step ぶん鳴らすか」しか返さない。ここがそれを
//! lane の pattern へ写して譜面にする。

use rand::RngExt;

use cmrt_rhythm::{generate_drum, DrumPattern, DrumRole};

use super::{GridInstance, GridState, GRID_STEPS};

/// drum 行が始まる instance index。行1=chord・行2=bass・行3=アルペジオの次。
pub const FIRST_DRUM_ROW: usize = 3;

/// drum 行を4 role ぶん揃えられる最小の track 数。
///
/// `t` キーの巡回にこの数（7）を足してあるのは、chord mode の3行に drum 4 role を
/// 過不足なく足した構成を1押しで出すため。
pub const FULL_DRUM_TRACK_COUNT: usize = FIRST_DRUM_ROW + DrumRole::ALL.len();

/// drum 行を1つだけ持つ track 数。役割は抽選になる。
const SINGLE_DRUM_TRACK_COUNT: usize = FIRST_DRUM_ROW + 1;

/// drum 行の音高。
///
/// drum の patch は音高で音色そのものが変わるので、抽選もコードへの寄せもせずここで固定する。
/// 動かせるのは NOTE 欄の wheel だけ。
const DRUM_NOTE: u8 = super::DEFAULT_NOTE;

/// `track_count` の構成で `index` 行が担当する drum role。drum 行でなければ `None`。
///
/// 画面は instance 昇順に上から描かれるので、下から kick・snare・hi-hat・percussion に
/// なるよう [`DrumRole::ALL`] をそのまま index へ当てる。
///
/// track 4 のときだけ drum 行が1つしか無く役割が定まらない。`current` があればそれを保ち、
/// 無ければ抽選する。入り直しや `t` の往復のたびに役割が変わらないための引数。
pub(super) fn drum_role_for(
    track_count: usize,
    index: usize,
    current: Option<DrumRole>,
    rng: &mut impl RngExt,
) -> Option<DrumRole> {
    if index < FIRST_DRUM_ROW || track_count < SINGLE_DRUM_TRACK_COUNT {
        return None;
    }
    if track_count == SINGLE_DRUM_TRACK_COUNT {
        return (index == FIRST_DRUM_ROW).then(|| current.unwrap_or_else(|| random_role(rng)));
    }
    // 4 role を使い切ったあと（track 8 / 16 の行8以降）は従来どおりの自由な行。
    DrumRole::ALL.get(index - FIRST_DRUM_ROW).copied()
}

/// instance 群の drum 行を track 数から決め直す。
///
/// 役割が変わった行だけ譜面を書き直す。変わっていない行の手編集を消さないため。
/// 逆に drum を降ろした行は譜面をそのまま残す（次の引き直しで普通の行として引かれる）。
///
/// 役割が変わらなくても [`GridInstance::set_drum_role`] は毎回通す。保存値の
/// `lane_mode` を信じないという [`super::session`] の方針をここでも守るため。
pub(super) fn apply_drum_roles(instances: &mut [GridInstance]) {
    let track_count = instances.len();
    let mut rng = rand::rng();
    for (index, instance) in instances.iter_mut().enumerate() {
        let role = drum_role_for(track_count, index, instance.drum, &mut rng);
        let changed = role != instance.drum;
        instance.set_drum_role(index, role);
        if let Some(role) = role.filter(|_| changed) {
            instance.lanes[0].base_note = DRUM_NOTE;
            randomize_drum_pattern(instance, role, &mut rng);
        }
    }
}

/// drum 行の譜面を、役割に合った型で引き直す。
pub(super) fn randomize_drum_pattern(
    instance: &mut GridInstance,
    role: DrumRole,
    rng: &mut impl RngExt,
) {
    write_drum_pattern(instance, DrumPattern::random_for(role, rng));
}

/// 指定した型の譜面を drum 行の lane 0 へ書く。音高は動かさない。
/// 譜面が変わったときだけ true。
pub(super) fn write_drum_pattern(instance: &mut GridInstance, pattern: DrumPattern) -> bool {
    let Some(lane) = instance.lanes.first_mut() else {
        return false;
    };
    let before = lane.pattern.clone();
    lane.pattern.clear();
    for hit in generate_drum(pattern, GRID_STEPS) {
        let end = hit.step + hit.duration_steps.max(1) - 1;
        let _ = lane.pattern.draw_span(hit.step, end);
    }
    lane.pattern != before
}

fn random_role(rng: &mut impl RngExt) -> DrumRole {
    DrumRole::ALL[rng.random_range(0..DrumRole::ALL.len())]
}

impl GridState {
    /// その instance が drum 行なら、担当する役割。
    pub fn drum_role(&self, instance: usize) -> Option<DrumRole> {
        self.instances.get(instance)?.drum
    }

    /// drum 行の譜面を指定した型で書き直す。変化が無ければ `false`。
    ///
    /// 型と行の役割が食い違っていたら何もしない（wheel が別の行の list を当てないための番人）。
    pub fn apply_drum_pattern(&mut self, instance: usize, pattern: DrumPattern) -> bool {
        let Some(item) = self.instances.get_mut(instance) else {
            return false;
        };
        if item.drum != Some(pattern.role()) || !write_drum_pattern(item, pattern) {
            return false;
        }
        self.refresh_lane_display_patterns();
        true
    }
}

#[cfg(test)]
mod tests;
