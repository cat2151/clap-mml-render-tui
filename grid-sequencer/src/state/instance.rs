//! CLAP instance と、その instance が共有する note lane の所有モデル。

use cmrt_rhythm::DrumRole;

use super::{NotePattern, DEFAULT_NOTE, SWING_MIN};

/// 初期検証で chord 構成音へ割り当てる voice 数。
pub const CHORD_VOICE_LANES: usize = 4;

/// bass 行の lane 数。lane 0 = コードの bass 音、lane 1 = その1オクターブ上。
pub const BASS_OCTAVE_LANES: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaneAddress {
    pub instance: usize,
    pub lane: usize,
}

impl LaneAddress {
    pub const fn new(instance: usize, lane: usize) -> Self {
        Self { instance, lane }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridLaneMode {
    #[default]
    Single,
    /// bass の root と octave 上の2声。[`super::BASS_ROW`] だけが持つ。
    BassOctave2,
    ChordVoices4,
    /// drum 行（[`super::drum`]）。lane は1本で、音高はコードへ寄せない。
    Drum,
}

impl GridLaneMode {
    pub const fn lane_count(self) -> usize {
        match self {
            Self::Single | Self::Drum => 1,
            Self::BassOctave2 => BASS_OCTAVE_LANES,
            Self::ChordVoices4 => CHORD_VOICE_LANES,
        }
    }

    /// 表示で高音を上へ反転する mode か。lane 0 が最低音であることが前提。
    pub const fn stacks_high_notes_on_top(self) -> bool {
        matches!(self, Self::BassOctave2 | Self::ChordVoices4)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridLane {
    pub base_note: u8,
    pub pattern: NotePattern,
}

impl Default for GridLane {
    fn default() -> Self {
        Self {
            base_note: DEFAULT_NOTE,
            pattern: NotePattern::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridInstance {
    pub patch: Option<String>,
    pub lane_mode: GridLaneMode,
    /// drum 行なら、その instance が担当する打楽器。役割は track 数から決まる
    /// （[`super::drum::drum_role_for`]）ので、ユーザーが直に編集する値ではない。
    pub drum: Option<DrumRole>,
    /// ChordVoices4の累積転回数。正は上方向、負は下方向へNOTE wheelで進む。
    pub voicing_rotation: i8,
    /// shuffle クオンタイズの量（百分率）。[`SWING_MIN`]〜[`SWING_MAX`]。
    ///
    /// 実際に効くかは譜面しだい（[`super::swing`]）。ここは「引いた値」であって
    /// 「効いている値」ではない。整数なのは [`GridInstance`] が `Eq` を要求するため
    /// （undo の差分比較が導出 `PartialEq` に乗っている）。
    pub swing: u8,
    pub lanes: Vec<GridLane>,
}

impl GridInstance {
    pub fn new(index: usize) -> Self {
        let lane_mode = default_lane_mode(index);
        Self {
            patch: None,
            lane_mode,
            drum: None,
            voicing_rotation: 0,
            swing: SWING_MIN,
            lanes: vec![GridLane::default(); lane_mode.lane_count()],
        }
    }

    /// drum 行かどうかを切り替える。lane の形も一緒に付け替える。
    ///
    /// `index` を取るのは、drum を降ろしたときに行番号どおりの mode へ戻すため。
    pub(crate) fn set_drum_role(&mut self, index: usize, role: Option<DrumRole>) {
        self.drum = role;
        self.lane_mode = match role {
            Some(_) => GridLaneMode::Drum,
            None => default_lane_mode(index),
        };
        if self.lane_mode != GridLaneMode::ChordVoices4 {
            // 転回は 4 voice の行だけが持つ状態。他の mode へ落とすときは一緒に捨てる。
            self.voicing_rotation = 0;
        }
        self.normalize();
    }

    /// 保存値の不足 lane を補い、mode の capacity を超えた lane を捨てる。
    pub fn normalize(&mut self) {
        let lane_count = self.lane_mode.lane_count();
        self.lanes.resize(lane_count, GridLane::default());
        self.lanes.truncate(lane_count);
    }
}

/// 行番号だけから決まる lane の形。drum 行はこの上から [`GridLaneMode::Drum`] で覆われる。
///
/// 行1 = chord、行2 = bass は chord mode が占有するので、4声コードの既定行は行3。
pub(crate) fn default_lane_mode(index: usize) -> GridLaneMode {
    match index {
        1 => GridLaneMode::BassOctave2,
        2 => GridLaneMode::ChordVoices4,
        _ => GridLaneMode::Single,
    }
}

impl Default for GridInstance {
    fn default() -> Self {
        Self::new(0)
    }
}

// 既存の同居テストをinstance modelへ段階移行するため、test build内だけprimary laneを
// 直接参照できるようにする。production APIにはrow互換面を残さない。
#[cfg(test)]
impl std::ops::Deref for GridInstance {
    type Target = GridLane;

    fn deref(&self) -> &Self::Target {
        &self.lanes[0]
    }
}

#[cfg(test)]
impl std::ops::DerefMut for GridInstance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lanes[0]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisibleRowKind {
    Normal,
    ChordSummary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibleNoteRow {
    pub address: LaneAddress,
    pub kind: VisibleRowKind,
}

#[cfg(test)]
mod tests;
