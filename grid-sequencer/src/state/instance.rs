//! CLAP instance と、その instance が共有する note lane の所有モデル。

use super::{NotePattern, DEFAULT_NOTE};

/// 初期検証で chord 構成音へ割り当てる voice 数。
pub const CHORD_VOICE_LANES: usize = 4;

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
    ChordVoices4,
}

impl GridLaneMode {
    pub const fn lane_count(self) -> usize {
        match self {
            Self::Single => 1,
            Self::ChordVoices4 => CHORD_VOICE_LANES,
        }
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
    /// ChordVoices4の累積転回数。正は上方向、負は下方向へNOTE wheelで進む。
    pub voicing_rotation: i8,
    pub lanes: Vec<GridLane>,
}

impl GridInstance {
    pub fn new(index: usize) -> Self {
        let lane_mode = if index == 1 {
            GridLaneMode::ChordVoices4
        } else {
            GridLaneMode::Single
        };
        Self {
            patch: None,
            lane_mode,
            voicing_rotation: 0,
            lanes: vec![GridLane::default(); lane_mode.lane_count()],
        }
    }

    /// 保存値の不足 lane を補い、mode の capacity を超えた lane を捨てる。
    pub fn normalize(&mut self) {
        let lane_count = self.lane_mode.lane_count();
        self.lanes.resize(lane_count, GridLane::default());
        self.lanes.truncate(lane_count);
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
