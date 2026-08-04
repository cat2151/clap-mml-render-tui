//! chord mode の再生状態。
//!
//! 抽選したコード進行を grid 1周（16ステップ = 全音符1個）ごとに1コードずつ進め、
//! [`CHORD_ROW`] の instance で和音として鳴らす。ほかの行はリズムをそのままに、
//! note number だけを現在のコードの構成音へ寄せる。
//!
//! コード進行そのものの抽選（カタログと rng）は画面側の仕事。ここは「与えられた
//! 進行をどう鳴らすか」だけを持つ。

use std::time::Instant;

use super::{GridRow, GridScheduledMessage, GridState};

/// 和音を鳴らす行。UI の行1 = realtime play server の instance 0。
pub const CHORD_ROW: usize = 0;

/// 抽選済みのコード進行と、その再生位置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordPlayback {
    key: &'static str,
    degrees: String,
    /// コード1つぶんの note number 群を進行の順に並べたもの。空にはしない。
    chords: Vec<Vec<u8>>,
    index: usize,
}

impl ChordPlayback {
    /// 空の進行は再生できないので `None` を返す。
    pub fn new(key: &'static str, degrees: String, chords: Vec<Vec<u8>>) -> Option<Self> {
        if chords.is_empty() {
            return None;
        }
        Some(Self {
            key,
            degrees,
            chords,
            index: 0,
        })
    }

    pub fn key(&self) -> &str {
        self.key
    }

    pub fn degrees(&self) -> &str {
        &self.degrees
    }

    /// 進行に含まれるコードの数。1以上であることが構築時に保証されている。
    pub fn chord_count(&self) -> usize {
        self.chords.len()
    }

    /// 進行内の現在位置（0 始まり）。
    pub fn index(&self) -> usize {
        self.index
    }

    /// いま鳴らすべき和音の note number 群。
    pub fn current(&self) -> &[u8] {
        &self.chords[self.index]
    }

    /// 次のコードへ進める。進行を1周して先頭へ戻ったときだけ true。
    fn advance(&mut self) -> bool {
        self.index = (self.index + 1) % self.chords.len();
        self.index == 0
    }

    /// いま鳴らしているのが進行の最後のコードか（= この小節が最終小節）。
    /// 待機 bank への先読みロードを始める合図に使う。
    pub(super) fn is_last(&self) -> bool {
        self.index + 1 == self.chords.len()
    }

    /// 和音に含まれるピッチクラス（0〜11）の集合。
    pub(super) fn pitch_classes(&self) -> [bool; 12] {
        let mut classes = [false; 12];
        for note in self.current() {
            classes[usize::from(note % 12)] = true;
        }
        classes
    }
}

impl GridState {
    /// chord mode を切り替える。`None` で解除。
    ///
    /// 鳴っている音を止める note off を返す。呼び出し側は必ず送ること
    /// （音色ロードを伴わない切替では消音が他に走らないため）。
    pub fn set_chord(
        &mut self,
        chord: Option<ChordPlayback>,
        now: Instant,
    ) -> Vec<GridScheduledMessage> {
        self.chord = chord;
        self.discard_pending_cycle();
        self.apply_chord_to_rows();
        self.refresh_lane_display_patterns();
        self.take_silence_messages(now)
    }

    /// grid を1周したときに次のコードへ進める。
    ///
    /// 進行を1周し終えたら、先読みロードが済んでいる次サイクルへ bank ごと差し替える
    /// （[`super::cycle`]）。差し替えは `attack_current_step()` の直前に起きるので、
    /// その小節の頭から新しい進行の1コード目がそのまま鳴る。
    ///
    /// シングルバッファリング（[`crate::single_buffer`]）では裏読みをしていないので
    /// 差し替えず、「鳴らしきった」合図だけを立てて `poll_steps` にクロックを畳ませる。
    pub(super) fn advance_chord(&mut self) {
        let Some(chord) = self.chord.as_mut() else {
            return;
        };
        if chord.advance() {
            if self.stop_at_cycle_end {
                self.cycle_wrapped = true;
                return;
            }
            self.commit_pending_cycle();
        }
        // 最終小節へ入った。画面側はここから次の抽選と先読みロードを始める。
        if self.chord.as_ref().is_some_and(ChordPlayback::is_last) {
            self.preload_due = true;
        }
        self.apply_chord_to_rows();
    }

    /// 和音以外の行の note number を、現在のコードの構成音へ寄せる。
    ///
    /// 構成音をそのまま割り当てると全行が和音と同じ狭い音域へ集まってしまうので、
    /// 抽選した `base_note` の高さを保ったまま、最も近い構成音へスナップする。
    /// chord mode を解除したときは `base_note` へ戻す。
    pub(super) fn apply_chord_to_rows(&mut self) {
        let classes = self.chord.as_ref().map(ChordPlayback::pitch_classes);
        apply_pitch_classes(&mut self.rows, classes);
    }
}

/// 行の並びの note number を、与えたコードの構成音へ寄せる。
///
/// `GridState` の外へ出してあるのは、chord mode が「鳴っている grid を触らずに
/// 次サイクルを抽選する」ために、複製した行の並びへ同じ処理をかけるため。
pub fn snap_rows_to_chord(rows: &mut [GridRow], chord: &ChordPlayback) {
    apply_pitch_classes(rows, Some(chord.pitch_classes()));
}

fn apply_pitch_classes(rows: &mut [GridRow], classes: Option<[bool; 12]>) {
    for (index, row) in rows.iter_mut().enumerate() {
        match classes {
            // 和音の行は `note` を使わない（`current()` の全構成音を鳴らす）。
            Some(_) if index == CHORD_ROW => {}
            Some(classes) => row.note = snap_to_chord(row.base_note, &classes),
            None => row.note = row.base_note,
        }
    }
}

/// `base` に最も近い、ピッチクラスが `classes` に含まれる note number を返す。
///
/// 同距離なら低いほうを選ぶ（上へ寄って音域が上ずるのを防ぐ）。
fn snap_to_chord(base: u8, classes: &[bool; 12]) -> u8 {
    if !classes.iter().any(|on| *on) {
        return base;
    }
    // 3和音以上ならピッチクラスの間隔は最大でも半音6個ぶんなので、距離6まで見れば必ず当たる。
    for distance in 0..=6 {
        if let Some(down) = base.checked_sub(distance) {
            if classes[usize::from(down % 12)] {
                return down;
            }
        }
        if let Some(up) = base.checked_add(distance) {
            if up <= 127 && classes[usize::from(up % 12)] {
                return up;
            }
        }
    }
    base
}

#[cfg(test)]
mod tests;
