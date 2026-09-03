//! Grid Sequencer の揮発履歴で選んだ1周を Daily DAW 全体へ置換する入口。

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{project::DawProjectSnapshot, DawApp, WorkspaceKind, FIRST_PLAYABLE_TRACK};

#[derive(Clone, Debug, PartialEq)]
pub struct DawGridImportSong {
    pub bpm: f64,
    pub chord: Option<DawGridChordSource>,
    pub tracks: Vec<DawGridImportTrack>,
    /// preview で先頭1小節を試聴済みなら、そのとき測った mixer 初期値（dB）。
    /// 未試聴なら `None` で、mixer は一律 0dB から始まる。
    /// 先頭に [`FIRST_PLAYABLE_TRACK`] ぶんの非演奏 track を含む DAW 側の並び。
    pub track_volumes_db: Option<Vec<i32>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DawGridImportTrack {
    pub patch: Option<String>,
    pub swing: u8,
    pub measures: Vec<String>,
    pub chord_binding: Option<DawGridChordBinding>,
}

/// Grid から受け取る、再編集可能な chord track と exact-voicing hint。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DawGridChordSource {
    pub init: String,
    pub measures: Vec<String>,
    pub voicings: Vec<DawGridChordVoicing>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DawGridChordVoicing {
    pub bass: Option<u8>,
    pub notes: Vec<u8>,
}

/// chord track から演奏 track の音高を解決する方法。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DawGridChordBinding {
    Chord,
    Bass {
        lanes: Vec<DawGridLane>,
    },
    Arpeggio {
        rotation: i8,
        lanes: Vec<DawGridLane>,
    },
    NearestChordTone {
        lanes: Vec<DawGridLane>,
    },
}

impl DawGridChordBinding {
    pub(crate) const fn label(&self) -> &'static str {
        match self {
            Self::Chord => "grid chord",
            Self::Bass { .. } => "grid bass",
            Self::Arpeggio { .. } => "grid arp",
            Self::NearestChordTone { .. } => "grid chord tone",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DawGridLane {
    pub base_note: u8,
    pub steps: Vec<DawGridNoteStep>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DawGridNoteStep {
    #[default]
    Rest,
    Attack,
    Tie,
}

/// init JSONへ永続化する生成レシピ。Daily recovery後もchordへのリンクを残す。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DawGridChordGeneration {
    version: u8,
    pub(crate) source: DawGridChordSource,
    pub(crate) binding: DawGridChordBinding,
}

impl DawGridChordGeneration {
    const VERSION: u8 = 1;

    fn new(source: DawGridChordSource, binding: DawGridChordBinding) -> Self {
        Self {
            version: Self::VERSION,
            source,
            binding,
        }
    }

    pub(crate) fn from_json(value: &Value) -> Option<Self> {
        let recipe: Self = serde_json::from_value(value.clone()).ok()?;
        (recipe.version == Self::VERSION).then_some(recipe)
    }
}

impl DawApp {
    /// Daily DAW の既存内容を、Grid の1周だけで全置換する。
    pub fn replace_with_grid_song(&mut self, song: DawGridImportSong) -> Result<()> {
        if self.workspace_kind != WorkspaceKind::Daily {
            bail!("Grid history は Daily DAW にだけ import できます");
        }
        let bpm = song.bpm;
        // preview を聴かずに import した場合は測定値が無い。先頭小節の cache が
        // 出そろってから mixer 初期値を決め直す。
        let needs_auto_trim = song.track_volumes_db.is_none();
        let snapshot = grid_song_snapshot(song)?;
        let tracks = snapshot.tracks;
        let measures = snapshot.measures;
        self.apply_project_snapshot_for_recovery(snapshot);
        if needs_auto_trim {
            self.request_auto_trim_from_first_measure();
        }
        self.editor.cursor_track = FIRST_PLAYABLE_TRACK;
        self.editor.cursor_measure = 1;
        self.save();
        self.kick_all_pending();
        self.append_log_line(format!(
            "Grid history を全置換import: {} tracks / {} measures / BPM {:.0}",
            tracks - FIRST_PLAYABLE_TRACK,
            measures,
            bpm,
        ));
        Ok(())
    }
}

pub(crate) fn grid_song_snapshot(song: DawGridImportSong) -> Result<DawProjectSnapshot> {
    if !song.bpm.is_finite() || song.bpm <= 0.0 {
        bail!("Grid history の BPM が不正です");
    }
    if song.tracks.is_empty() {
        bail!("Grid history に track がありません");
    }

    let DawGridImportSong {
        bpm,
        chord,
        tracks: imported_tracks,
        track_volumes_db,
    } = song;
    if let Some(chord) = &chord {
        if chord.measures.is_empty() || chord.voicings.len() != chord.measures.len() {
            bail!("Grid history の chord progression が不正です");
        }
    }

    let measures = imported_tracks
        .iter()
        .map(|track| track.measures.len())
        .chain(chord.iter().map(|chord| chord.measures.len()))
        .max()
        .unwrap_or(1)
        .max(1);
    let tracks = FIRST_PLAYABLE_TRACK + imported_tracks.len();
    let mut data = vec![vec![String::new(); measures + 1]; tracks];
    data[0][0] = format!(r#"{{"beat":"4/4"}}t{bpm:.0}"#);

    if let Some(chord) = &chord {
        data[crate::CHORD_TRACK][0] = chord.init.clone();
        for (measure, cell) in chord.measures.iter().enumerate() {
            data[crate::CHORD_TRACK][measure + 1] = cell.clone();
        }
    }

    for (index, track) in imported_tracks.into_iter().enumerate() {
        let daw_track = FIRST_PLAYABLE_TRACK + index;
        let generation = chord
            .as_ref()
            .cloned()
            .zip(track.chord_binding)
            .map(|(source, binding)| DawGridChordGeneration::new(source, binding));
        data[daw_track][0] =
            track_init_mml(track.patch.as_deref(), track.swing, generation.as_ref());
        if generation.is_none() {
            for (measure, mml) in track.measures.into_iter().enumerate() {
                data[daw_track][measure + 1] = mml;
            }
        }
    }

    // 長さが合わない測定値は、track 構成が変わったものとみなして捨てる。
    let track_volumes_db = track_volumes_db
        .filter(|volumes_db| volumes_db.len() == tracks)
        .unwrap_or_else(|| vec![0; tracks]);

    Ok(DawProjectSnapshot {
        data,
        track_volumes_db,
        tracks,
        measures,
    })
}

fn track_init_mml(
    patch: Option<&str>,
    swing: u8,
    generation: Option<&DawGridChordGeneration>,
) -> String {
    let mut json = Map::new();
    if let Some(patch) = patch {
        json.insert(
            "Surge XT patch".to_string(),
            Value::String(patch.to_string()),
        );
    }
    // 現行DAW rendererはこの値を解釈しないが、Grid固有値を落とさずセルへ保持する。
    json.insert("swing".to_string(), Value::from(swing));
    if let Some(generation) = generation {
        json.insert(
            crate::mml::chord_generation::GENERATE_FROM_CHORD_TRACK_KEY.to_string(),
            serde_json::to_value(generation).expect("grid chord recipe is serializable"),
        );
    }
    Value::Object(json).to_string()
}

#[cfg(test)]
mod tests;
