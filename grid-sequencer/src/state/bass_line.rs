//! ベースラインを bass 行（[`BASS_ROW`]）の lane 群へ書き込むドメイン API。
//!
//! [`cmrt_arpeggiator`] は「どの step で、どちらの声部を、何 step ぶん鳴らすか」しか
//! 返さない。ここが声部を lane へ写して譜面にする。lane の音高は保存値ではなく
//! 現在のコードの bass 音から導出される（[`super::chord`]）ので、書き込むのは pattern だけ。
//!
//! 声部 0 = lane 0 = bass 音、声部 1 = lane 1 = その1オクターブ上。表示は上下が
//! 反転するが（[`super::GridState::visible_note_rows`]）、格納上の lane index は反転しない。

use cmrt_arpeggiator::BassNote;

use super::{GridInstance, GridState, BASS_ROW};

/// bass 行の全 lane の pattern を捨て、ベースラインを書き直す。変化があれば true。
///
/// [`GridState`] を経由しない形にしてあるのは、1周ごとの抽選が「鳴っている grid を
/// 触らずに複製の上で引く」ため（[`super::randomize`]）。
pub(super) fn write_bass_line(instance: &mut GridInstance, notes: &[BassNote]) -> bool {
    let before = instance.lanes.clone();
    for lane in &mut instance.lanes {
        lane.pattern.clear();
    }
    // step 昇順に描くことで、`draw_span` の「新しい Attack が既存 note の Tie 上へ
    // 来たら前の note を切り詰める」挙動が打ち切りの安全網になる。
    let mut notes = notes.to_vec();
    notes.sort_by_key(|note| note.step);
    for note in notes {
        let Some(lane) = instance.lanes.get_mut(note.voice) else {
            continue;
        };
        let end = note.step + note.duration_steps.max(1) - 1;
        let _ = lane.pattern.draw_span(note.step, end);
    }
    instance.lanes != before
}

impl GridState {
    /// bass 行にベースラインを書けるか。chord OFF では音高が導出できないので書けない。
    pub fn bass_line_is_available(&self) -> bool {
        self.chord.is_some() && self.instances.len() > BASS_ROW
    }

    /// bass 行の全 lane の pattern を捨て、ベースラインを書き直す。
    ///
    /// lane が無い `voice` は無視する（鳴らしようがない）。
    /// 変化がなければ `false` を返し、表示の作り直しもしない。
    pub fn apply_bass_line(&mut self, notes: &[BassNote]) -> bool {
        if !self.bass_line_is_available() {
            return false;
        }
        let Some(item) = self.instances.get_mut(BASS_ROW) else {
            return false;
        };
        if !write_bass_line(item, notes) {
            return false;
        }
        self.refresh_lane_display_patterns();
        true
    }
}

#[cfg(test)]
mod tests;
