//! Grid Sequencer の永続化用 wire DTO。
//!
//! domain crate には依存せず、instance/lane形式と旧rows形式をここで正規化する。

use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

const GRID_STEPS: usize = 16;
const DEFAULT_NOTE: u8 = 60;
const CHORD_VOICE_LANES: usize = 4;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GridSequencerSessionState {
    pub instances: Vec<GridSequencerInstanceState>,
    pub pattern_evolution: GridPatternEvolutionState,
}

impl Serialize for GridSequencerSessionState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("GridSequencerSessionState", 2)?;
        state.serialize_field("instances", &self.instances)?;
        state.serialize_field("pattern_evolution", &self.pattern_evolution)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for GridSequencerSessionState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let Some(object) = value.as_object() else {
            return Ok(Self::default());
        };
        let pattern_evolution = object
            .get("pattern_evolution")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        // fieldが存在すれば、空・壊れた配列でもlegacy rowsより優先する。
        let instances = match object.get("instances") {
            Some(value) => deserialize_instances(value),
            None => migrate_legacy_rows(object.get("rows")),
        };
        Ok(Self {
            instances,
            pattern_evolution,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GridSequencerInstanceState {
    pub patch: Option<String>,
    pub lane_mode: GridLaneModeState,
    pub voicing_rotation: i8,
    pub lanes: Vec<GridSequencerLaneState>,
}

impl GridSequencerInstanceState {
    fn normalize(&mut self) {
        let count = self.lane_mode.lane_count();
        self.lanes.resize(count, GridSequencerLaneState::default());
        self.lanes.truncate(count);
    }
}

impl Default for GridSequencerInstanceState {
    fn default() -> Self {
        Self {
            patch: None,
            lane_mode: GridLaneModeState::Single,
            voicing_rotation: 0,
            lanes: vec![GridSequencerLaneState::default()],
        }
    }
}

impl<'de> Deserialize<'de> for GridSequencerInstanceState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let Some(object) = value.as_object() else {
            return Ok(Self::default());
        };
        let patch = parse_patch(object.get("patch"));
        let lane_mode = object
            .get("lane_mode")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        let voicing_rotation = object
            .get("voicing_rotation")
            .and_then(Value::as_i64)
            .and_then(|value| i8::try_from(value).ok())
            .unwrap_or(0);
        let lanes = object
            .get("lanes")
            .and_then(Value::as_array)
            .map(|lanes| {
                lanes
                    .iter()
                    .filter_map(|lane| serde_json::from_value(lane.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        let mut result = Self {
            patch,
            lane_mode,
            voicing_rotation,
            lanes,
        };
        result.normalize();
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GridLaneModeState {
    #[default]
    Single,
    ChordVoices4,
}

impl GridLaneModeState {
    fn lane_count(self) -> usize {
        match self {
            Self::Single => 1,
            Self::ChordVoices4 => CHORD_VOICE_LANES,
        }
    }
}

impl<'de> Deserialize<'de> for GridLaneModeState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(match value.as_str() {
            Some("chord_voices4") => Self::ChordVoices4,
            _ => Self::Single,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridSequencerLaneState {
    pub base_note: u8,
    pub note_steps: Vec<GridNoteStepState>,
}

impl Serialize for GridSequencerLaneState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut note_steps = self.note_steps.clone();
        normalize_note_steps(&mut note_steps);
        let mut lane = serializer.serialize_struct("GridSequencerLaneState", 2)?;
        lane.serialize_field("base_note", &self.base_note)?;
        lane.serialize_field("note_steps", &note_steps)?;
        lane.end()
    }
}

impl Default for GridSequencerLaneState {
    fn default() -> Self {
        Self {
            base_note: DEFAULT_NOTE,
            note_steps: rest_steps(),
        }
    }
}

impl<'de> Deserialize<'de> for GridSequencerLaneState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(parse_lane(&value))
    }
}

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GridNoteStepState {
    #[default]
    Rest,
    Attack,
    Tie,
}

impl<'de> Deserialize<'de> for GridNoteStepState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(note_step_from_value(&value))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GridPatternEvolutionState {
    #[default]
    Auto,
    Hold,
}

impl<'de> Deserialize<'de> for GridPatternEvolutionState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(match value.as_str() {
            Some("hold") => Self::Hold,
            _ => Self::Auto,
        })
    }
}

fn deserialize_instances(value: &Value) -> Vec<GridSequencerInstanceState> {
    value
        .as_array()
        .map(|instances| {
            instances
                .iter()
                .filter_map(|instance| serde_json::from_value(instance.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn migrate_legacy_rows(value: Option<&Value>) -> Vec<GridSequencerInstanceState> {
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
                voicing_rotation: 0,
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

fn parse_patch(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|patch| !patch.is_empty())
        .map(str::to_owned)
}

fn parse_lane(value: &Value) -> GridSequencerLaneState {
    let Some(object) = value.as_object() else {
        return GridSequencerLaneState::default();
    };
    let base_note = object
        .get("base_note")
        .and_then(Value::as_i64)
        .unwrap_or(i64::from(DEFAULT_NOTE))
        .clamp(0, 127) as u8;
    let note_steps = match object.get("note_steps") {
        Some(value) => deserialize_note_steps(value),
        None => migrate_legacy_pattern(object.get("cells"), object.get("duration")),
    };
    GridSequencerLaneState {
        base_note,
        note_steps,
    }
}

fn rest_steps() -> Vec<GridNoteStepState> {
    vec![GridNoteStepState::Rest; GRID_STEPS]
}

fn note_step_from_value(value: &Value) -> GridNoteStepState {
    match value.as_str() {
        Some("attack") => GridNoteStepState::Attack,
        Some("tie") => GridNoteStepState::Tie,
        _ => GridNoteStepState::Rest,
    }
}

fn deserialize_note_steps(value: &Value) -> Vec<GridNoteStepState> {
    let mut steps = value
        .as_array()
        .map(|items| {
            items
                .iter()
                .take(GRID_STEPS)
                .map(note_step_from_value)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    steps.resize(GRID_STEPS, GridNoteStepState::Rest);
    normalize_ties(&mut steps);
    steps
}

fn migrate_legacy_pattern(
    cells: Option<&Value>,
    duration: Option<&Value>,
) -> Vec<GridNoteStepState> {
    let attacks = cells
        .and_then(Value::as_array)
        .map(|items| {
            let mut result = items
                .iter()
                .take(GRID_STEPS)
                .map(|item| item.as_bool().unwrap_or(false))
                .collect::<Vec<_>>();
            result.resize(GRID_STEPS, false);
            result
        })
        .unwrap_or_else(|| vec![false; GRID_STEPS]);
    let length = match duration.and_then(Value::as_str) {
        Some("quarter") | Some("1/4") => 4,
        Some("whole") | Some("1/1") => GRID_STEPS,
        _ => 1,
    };
    let mut steps = rest_steps();
    for (step, attack) in attacks.iter().enumerate() {
        if *attack {
            steps[step] = GridNoteStepState::Attack;
        }
    }
    for (attack, present) in attacks.iter().enumerate() {
        if !present {
            continue;
        }
        for step in attack + 1..(attack + length).min(GRID_STEPS) {
            if attacks[step] {
                break;
            }
            steps[step] = GridNoteStepState::Tie;
        }
    }
    steps
}

fn normalize_note_steps(steps: &mut Vec<GridNoteStepState>) {
    steps.truncate(GRID_STEPS);
    steps.resize(GRID_STEPS, GridNoteStepState::Rest);
    normalize_ties(steps);
}

fn normalize_ties(steps: &mut [GridNoteStepState]) {
    let mut sounding = false;
    for step in steps {
        match *step {
            GridNoteStepState::Rest => sounding = false,
            GridNoteStepState::Attack => sounding = true,
            GridNoteStepState::Tie if !sounding => *step = GridNoteStepState::Rest,
            GridNoteStepState::Tie => {}
        }
    }
}

#[cfg(test)]
mod tests;
