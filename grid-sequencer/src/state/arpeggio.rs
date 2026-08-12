//! アルペジオを instance の lane 群へ書き込むドメイン API。
//!
//! [`cmrt_arpeggiator`] は「何番目の声部を、どの step で、何 step ぶん鳴らすか」しか
//! 返さない。ここが声部を lane へ写して譜面にする。lane の音高は保存値ではなく
//! 現在のコードから導出される（[`super::chord`]）ので、書き込むのは pattern だけ。
//!
//! 声部 0 = lane 0 = 最低音。表示は `ChordVoices4` のとき上下が反転するが
//! （[`super::GridState::visible_note_rows`]）、格納上の lane index は反転しない。

use cmrt_arpeggiator::ArpNote;

use super::{GridInstance, GridState, LaneAddress, BASS_ROW, CHORD_ROW};

/// instance の全 lane の pattern を捨て、アルペジオを書き直す。変化があれば true。
///
/// [`GridState`] を経由しない形にしてあるのは、1周ごとの抽選が「鳴っている grid を
/// 触らずに複製の上で引く」ため（[`super::randomize`]）。
pub(super) fn write_arpeggio(instance: &mut GridInstance, notes: &[ArpNote]) -> bool {
    let before = instance.lanes.clone();
    for lane in &mut instance.lanes {
        lane.pattern.clear();
    }
    // step 昇順に描くことで、`draw_span` の「新しい Attack が既存 note の Tie 上へ
    // 来たら前の note を切り詰める」挙動が打ち切りの二重の安全網になる。
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
    /// アルペジオに使える声部数。`resolved_note` が音を返す lane を数える。
    ///
    /// chord OFF・[`CHORD_ROW`]・[`BASS_ROW`]・Single lane の instance では 2 未満に
    /// なるので、呼び出し側はこれで対象外を弾ける。
    pub fn arp_voice_count(&self, instance: usize) -> usize {
        if self.chord.is_none() || instance == CHORD_ROW || instance == BASS_ROW {
            return 0;
        }
        let Some(item) = self.instances.get(instance) else {
            return 0;
        };
        (0..item.lanes.len())
            .filter(|lane| {
                self.resolved_note(LaneAddress::new(instance, *lane))
                    .is_some()
            })
            .count()
    }

    /// instance の全 lane の pattern を捨て、アルペジオを書き直す。
    ///
    /// 声部数を超える `voice` は無視する（lane が無いので鳴らしようがない）。
    /// 変化がなければ `false` を返し、表示の作り直しもしない。
    pub fn apply_arpeggio(&mut self, instance: usize, notes: &[ArpNote]) -> bool {
        if self.chord.is_none() || instance == CHORD_ROW || instance == BASS_ROW {
            return false;
        }
        let Some(item) = self.instances.get_mut(instance) else {
            return false;
        };
        if !write_arpeggio(item, notes) {
            return false;
        }
        self.refresh_lane_display_patterns();
        true
    }
}

#[cfg(test)]
mod tests;
