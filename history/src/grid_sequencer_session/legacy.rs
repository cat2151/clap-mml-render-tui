//! 旧 rows 形式（instance / lane を持たなかった頃）の読み取りと移行。
//!
//! 現行の instance 形式へ寄せるのはここだけの仕事。新形式の DTO へ旧形式の知識を
//! 混ぜないために module を分けてある。

use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::{
    normalize_note_steps, parse_lane, parse_patch, GridLaneModeState, GridNoteStepState,
    GridSequencerInstanceState, GridSequencerLaneState,
};

/// 旧rows形式の読み書きテストとmigration入力に使うDTO。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridSequencerRowState {
    pub patch: Option<String>,
    pub base_note: u8,
    pub note_steps: Vec<GridNoteStepState>,
}

impl Serialize for GridSequencerRowState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut note_steps = self.note_steps.clone();
        normalize_note_steps(&mut note_steps);
        let mut row = serializer.serialize_struct("GridSequencerRowState", 3)?;
        row.serialize_field("patch", &self.patch)?;
        row.serialize_field("base_note", &self.base_note)?;
        row.serialize_field("note_steps", &note_steps)?;
        row.end()
    }
}

impl Default for GridSequencerRowState {
    fn default() -> Self {
        let lane = GridSequencerLaneState::default();
        Self {
            patch: None,
            base_note: lane.base_note,
            note_steps: lane.note_steps,
        }
    }
}

impl<'de> Deserialize<'de> for GridSequencerRowState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let lane = parse_lane(&value);
        let patch = value
            .as_object()
            .and_then(|object| parse_patch(object.get("patch")));
        Ok(Self {
            patch,
            base_note: lane.base_note,
            note_steps: lane.note_steps,
        })
    }
}

/// 旧 rows 形式を instance 形式へ移す。
pub(super) fn migrate_legacy_rows(value: Option<&Value>) -> Vec<GridSequencerInstanceState> {
    let Some(rows) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| serde_json::from_value::<GridSequencerRowState>(row.clone()).ok())
        .enumerate()
        .map(|(index, row)| {
            let lane_mode = if index == 1 {
                GridLaneModeState::ChordVoices4
            } else {
                GridLaneModeState::Single
            };
            let mut instance = GridSequencerInstanceState {
                patch: row.patch,
                lane_mode,
                // 旧形式には drum 行が無い。役割は復元後に track 数から当たる。
                drum: None,
                voicing_rotation: 0,
                // 旧形式には swing が無い。跳ねなしから始める。
                swing: super::SWING_MIN,
                lanes: vec![GridSequencerLaneState {
                    base_note: row.base_note,
                    note_steps: row.note_steps,
                }],
            };
            instance.normalize();
            instance
        })
        .collect()
}
