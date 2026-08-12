//! Grid Sequencer の永続化用 wire DTO。
//!
//! domain crate には依存せず、instance/lane形式と旧rows形式をここで正規化する。

use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

mod legacy;

use legacy::migrate_legacy_rows;
pub use legacy::GridSequencerRowState;

const GRID_STEPS: usize = 16;
const DEFAULT_NOTE: u8 = 60;
const CHORD_VOICE_LANES: usize = 4;
const BASS_OCTAVE_LANES: usize = 2;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GridSequencerSessionState {
    pub instances: Vec<GridSequencerInstanceState>,
    pub cycle_random: GridCycleRandomState,
}

impl Serialize for GridSequencerSessionState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("GridSequencerSessionState", 2)?;
        state.serialize_field("instances", &self.instances)?;
        state.serialize_field("cycle_random", &self.cycle_random)?;
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
        let cycle_random = match object.get("cycle_random") {
            Some(value) => deserialize_cycle_random(value),
            // 6項目へ分解する前のセッション。AUTO/HOLD の2値から移行する。
            None => migrate_pattern_evolution(object.get("pattern_evolution")),
        };
        // fieldが存在すれば、空・壊れた配列でもlegacy rowsより優先する。
        let instances = match object.get("instances") {
            Some(value) => deserialize_instances(value),
            None => migrate_legacy_rows(object.get("rows")),
        };
        Ok(Self {
            instances,
            cycle_random,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GridSequencerInstanceState {
    pub patch: Option<String>,
    pub lane_mode: GridLaneModeState,
    /// drum 行なら、その instance が担当する打楽器。
    ///
    /// 役割は track 数から決まるが、track 4 のときだけ抽選になる。抽選結果をここへ
    /// 残しておかないと、起動のたびに役割（＝ patch とリズム）が変わってしまう。
    /// drum が無い頃のセッションには field ごと無い。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drum: Option<GridDrumRoleState>,
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
            drum: None,
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
        let drum = object.get("drum").and_then(parse_drum_role);
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
            drum,
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
    BassOctave2,
    ChordVoices4,
}

impl GridLaneModeState {
    fn lane_count(self) -> usize {
        match self {
            Self::Single => 1,
            Self::BassOctave2 => BASS_OCTAVE_LANES,
            Self::ChordVoices4 => CHORD_VOICE_LANES,
        }
    }
}

/// drum 行が担当する打楽器。domain の `DrumRole` に対応する wire DTO。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GridDrumRoleState {
    Kick,
    Snare,
    HiHat,
    Percussion,
}

/// 知らない値は「drum 行ではない」として捨てる。役割は track 数から当て直されるので、
/// 落としてもリズムが消えるだけで壊れない。
fn parse_drum_role(value: &Value) -> Option<GridDrumRoleState> {
    match value.as_str()? {
        "kick" => Some(GridDrumRoleState::Kick),
        "snare" => Some(GridDrumRoleState::Snare),
        "hi_hat" => Some(GridDrumRoleState::HiHat),
        "percussion" => Some(GridDrumRoleState::Percussion),
        _ => None,
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
            Some("bass_octave2") => Self::BassOctave2,
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

/// コード進行1周ごとに引き直す対象。既定は全 ON。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct GridCycleRandomState {
    pub patch: bool,
    pub note: bool,
    pub drum: bool,
    pub arp: bool,
    pub chord: bool,
    pub bpm: bool,
}

impl Default for GridCycleRandomState {
    fn default() -> Self {
        Self {
            patch: true,
            note: true,
            drum: true,
            arp: true,
            chord: true,
            bpm: true,
        }
    }
}

/// 欠けた field は既定（ON）として読む。項目を足したセッションを古い版が書き戻しても、
/// 足した項目が黙って OFF にならないため。
fn deserialize_cycle_random(value: &Value) -> GridCycleRandomState {
    let default = GridCycleRandomState::default();
    let Some(object) = value.as_object() else {
        return default;
    };
    let flag = |name: &str, fallback: bool| {
        object
            .get(name)
            .and_then(Value::as_bool)
            .unwrap_or(fallback)
    };
    GridCycleRandomState {
        patch: flag("patch", default.patch),
        note: flag("note", default.note),
        drum: flag("drum", default.drum),
        arp: flag("arp", default.arp),
        chord: flag("chord", default.chord),
        bpm: flag("bpm", default.bpm),
    }
}

/// 旧 `pattern_evolution`（AUTO / HOLD）からの移行。
///
/// HOLD は「譜面も音色も据え置き、進行とテンポだけ動かす」だったので、
/// 譜面まわりの4項目だけを OFF にする。
fn migrate_pattern_evolution(value: Option<&Value>) -> GridCycleRandomState {
    if value.and_then(Value::as_str) != Some("hold") {
        return GridCycleRandomState::default();
    }
    GridCycleRandomState {
        patch: false,
        note: false,
        drum: false,
        arp: false,
        chord: true,
        bpm: true,
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
