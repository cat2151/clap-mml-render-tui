//! Gridの1周を、Daily DAWへ渡せるsemantic source・生成recipe・MMLへ変換する。

use crate::{
    state::resolved_note_from, ChordPlayback, GridInstance, GridLane, GridLaneMode, LaneAddress,
    NotePattern, NoteStep, ARPEGGIO_ROW, BASS_ROW, CHORD_ROW, GRID_STEPS,
};

/// Grid の1周ぶんを再現する、揮発性のスナップショット。
#[derive(Clone, Debug, PartialEq)]
pub struct GridSongSnapshot {
    bpm: f64,
    instances: Vec<GridInstance>,
    chord: Option<ChordPlayback>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridDawTrack {
    pub patch: Option<String>,
    pub swing: u8,
    pub measures: Vec<String>,
    pub chord_binding: Option<GridDawChordBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridDawChordSource {
    pub init: String,
    pub measures: Vec<String>,
    pub voicings: Vec<GridDawChordVoicing>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridDawChordVoicing {
    pub bass: Option<u8>,
    pub notes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridDawChordBinding {
    Chord,
    Bass {
        lanes: Vec<GridDawLane>,
    },
    Arpeggio {
        rotation: i8,
        lanes: Vec<GridDawLane>,
    },
    NearestChordTone {
        lanes: Vec<GridDawLane>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridDawLane {
    pub base_note: u8,
    pub steps: Vec<NoteStep>,
}

impl GridSongSnapshot {
    pub(crate) fn new(
        bpm: f64,
        instances: Vec<GridInstance>,
        chord: Option<ChordPlayback>,
    ) -> Self {
        Self {
            bpm,
            instances,
            chord: chord.map(|chord| chord.restarted()),
        }
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    pub fn track_count(&self) -> usize {
        self.instances.len()
    }

    pub fn measure_count(&self) -> usize {
        self.chord.as_ref().map_or(1, ChordPlayback::chord_count)
    }

    pub fn chord_label(&self) -> Option<String> {
        self.chord
            .as_ref()
            .map(|chord| format!("{} {}", chord.key(), chord.degrees()))
    }

    pub fn daw_chord_source(&self) -> Option<GridDawChordSource> {
        let chord = self.chord.as_ref()?;
        let input = format!("key:{} {}", chord.key(), chord.degrees());
        let parsed = cmrt_chord::parse_chord_progression(&input).ok()?;
        if parsed.chord_texts().len() != chord.chord_count() {
            return None;
        }
        let voicings = (0..chord.chord_count())
            .filter_map(|index| chord.voicing_at(index))
            .map(|voicing| GridDawChordVoicing {
                bass: voicing.bass,
                notes: voicing.notes,
            })
            .collect::<Vec<_>>();
        if voicings.len() != chord.chord_count() {
            return None;
        }
        Some(GridDawChordSource {
            init: format!("key:{}", parsed.key_name()),
            measures: parsed.chord_texts().to_vec(),
            voicings,
        })
    }

    /// Gridの各instanceを、同じ番号のDaily DAW trackへ変換する。
    pub fn daw_tracks(&self) -> Vec<GridDawTrack> {
        let chord_is_exportable = self.daw_chord_source().is_some();
        self.instances
            .iter()
            .enumerate()
            .map(|(instance_index, instance)| GridDawTrack {
                patch: instance.patch.clone(),
                swing: instance.swing,
                measures: (0..self.measure_count())
                    .map(|measure| self.instance_measure_mml(instance_index, instance, measure))
                    .collect(),
                chord_binding: chord_is_exportable
                    .then(|| chord_binding(instance_index, instance))
                    .flatten(),
            })
            .collect()
    }

    fn instance_measure_mml(
        &self,
        instance_index: usize,
        instance: &GridInstance,
        measure: usize,
    ) -> String {
        let chord = self
            .chord
            .as_ref()
            .and_then(|chord| chord.at_index(measure));
        if instance_index == CHORD_ROW {
            if let Some(chord) = &chord {
                return chord_measure_mml(chord.current());
            }
        }
        instance
            .lanes
            .iter()
            .enumerate()
            .map(|(lane_index, lane)| {
                let note = resolved_note_from(
                    &self.instances,
                    chord.as_ref(),
                    LaneAddress::new(instance_index, lane_index),
                );
                lane_measure_mml(lane, note)
            })
            .collect::<Vec<_>>()
            .join(";")
    }
}

fn chord_binding(instance_index: usize, instance: &GridInstance) -> Option<GridDawChordBinding> {
    if instance_index == CHORD_ROW {
        return Some(GridDawChordBinding::Chord);
    }
    if instance.drum.is_some() {
        return None;
    }
    let lanes = instance
        .lanes
        .iter()
        .map(|lane| GridDawLane {
            base_note: lane.base_note,
            steps: lane.pattern.steps().to_vec(),
        })
        .collect();
    if instance_index == BASS_ROW {
        return Some(GridDawChordBinding::Bass { lanes });
    }
    if instance_index == ARPEGGIO_ROW || instance.lane_mode == GridLaneMode::ChordVoices4 {
        return Some(GridDawChordBinding::Arpeggio {
            rotation: instance.voicing_rotation,
            lanes,
        });
    }
    Some(GridDawChordBinding::NearestChordTone { lanes })
}

fn chord_measure_mml(notes: &[u8]) -> String {
    if notes.is_empty() {
        return "r1".to_string();
    }
    notes
        .iter()
        .map(|note| format!("{}1", note_name(*note)))
        .collect::<Vec<_>>()
        .join(";")
}

fn lane_measure_mml(lane: &GridLane, note: Option<u8>) -> String {
    let Some(note) = note else {
        return "r1".to_string();
    };
    pattern_mml(&lane.pattern, &note_name(note))
}

fn pattern_mml(pattern: &NotePattern, note: &str) -> String {
    let mut mml = String::new();
    let mut step = 0;
    while step < GRID_STEPS {
        match pattern.step(step).unwrap_or(NoteStep::Rest) {
            NoteStep::Attack => {
                let length = usize::from(pattern.attack_len(step).unwrap_or(1));
                append_span(&mut mml, note, length);
                step += length;
            }
            NoteStep::Rest | NoteStep::Tie => {
                let length = (step..GRID_STEPS)
                    .take_while(|index| pattern.step(*index) == Some(NoteStep::Rest))
                    .count()
                    .max(1);
                append_span(&mut mml, "r", length);
                step += length;
            }
        }
    }
    mml
}

fn append_span(mml: &mut String, value: &str, mut steps: usize) {
    const DURATIONS: &[(usize, &str)] = &[
        (16, "1"),
        (15, "2..."),
        (14, "2.."),
        (12, "2."),
        (8, "2"),
        (7, "4.."),
        (6, "4."),
        (4, "4"),
        (3, "8."),
        (2, "8"),
        (1, "16"),
    ];
    while steps > 0 {
        let (duration_steps, suffix) = DURATIONS
            .iter()
            .copied()
            .find(|(duration_steps, _)| *duration_steps <= steps)
            .expect("one sixteenth is always available");
        mml.push_str(value);
        mml.push_str(suffix);
        steps -= duration_steps;
    }
}

fn note_name(note: u8) -> String {
    const PITCHES: [&str; 12] = [
        "c", "c+", "d", "d+", "e", "f", "f+", "g", "g+", "a", "a+", "b",
    ];
    format!("o{}{}", note / 12, PITCHES[usize::from(note % 12)])
}
