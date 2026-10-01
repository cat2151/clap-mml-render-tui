//! サンプル MID モードの matrix。横 = 音のまとまり、縦 = 使われている音高（高い音が上）。
//! 音の段の下に、使われている KS・CC・pitch bend の段を並べ、まとまりごとに効いているものへ ● を付ける。

use std::collections::BTreeSet;

use ratatui::{style::Color, text::Line};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN},
};

use super::matrix::{row, visible_columns, NOTE_MARK, NO_NOTE_MARK, RULE_ON_MARK};
use super::{note_name, RuleGroup};
use crate::{keyswitch_name, SampleMidi, SampleMidiGroup};

const BEND_LABEL: &str = "pitch bend";
const EMPTY_TEXT: &str = "(鳴る音がありません)";

/// 段 1 つ。見出しと、まとまりごとに印を付けるか。
struct MidiRow {
    label: String,
    color: Color,
    /// 音高の段か（■ と · で描く）。でなければ KS・CC・bend の段（● と空白）。
    pitch: bool,
    on: Box<dyn Fn(&SampleMidiGroup) -> bool>,
}

fn rows(groups: &[SampleMidiGroup]) -> Vec<MidiRow> {
    let pitches: BTreeSet<u8> = groups.iter().flat_map(|g| g.pitches.clone()).collect();
    let keyswitches: BTreeSet<u8> = groups.iter().filter_map(|g| g.keyswitch).collect();
    let controllers: BTreeSet<u8> = groups.iter().flat_map(|g| g.controllers.clone()).collect();
    let mut out: Vec<MidiRow> = pitches
        .into_iter()
        .rev()
        .map(|pitch| MidiRow {
            label: note_name(pitch),
            color: MONOKAI_GRAY,
            pitch: true,
            on: Box::new(move |g| g.pitches.contains(&pitch)),
        })
        .collect();
    out.extend(keyswitches.into_iter().map(|ks| MidiRow {
        label: format!("KS {}", keyswitch_name(ks).unwrap_or_default()),
        color: RuleGroup::AttackRelease.color(),
        pitch: false,
        on: Box::new(move |g| g.keyswitch == Some(ks)),
    }));
    out.extend(controllers.into_iter().map(|cc| MidiRow {
        label: format!("CC{cc}"),
        color: RuleGroup::Row.color(),
        pitch: false,
        on: Box::new(move |g| g.controllers.contains(&cc)),
    }));
    if groups.iter().any(|g| g.bend) {
        out.push(MidiRow {
            label: BEND_LABEL.to_string(),
            color: RuleGroup::Pitch.color(),
            pitch: false,
            on: Box::new(|g| g.bend),
        });
    }
    out
}

fn groups(midi: &SampleMidi) -> Vec<SampleMidiGroup> {
    (0..midi.group_count())
        .filter_map(|group| midi.group(group))
        .collect()
}

/// 枠の内側に要る行数。
pub(super) fn height(midi: &SampleMidi) -> u16 {
    rows(&groups(midi)).len().max(1) as u16
}

pub(super) fn lines(midi: &SampleMidi, cursor: usize, width: u16) -> Vec<Line<'static>> {
    let groups = groups(midi);
    let rows = rows(&groups);
    if rows.is_empty() {
        return vec![Line::styled(EMPTY_TEXT, base_style().fg(MONOKAI_GRAY))];
    }
    let label_width = rows.iter().map(|r| r.label.len() + 1).max().unwrap_or(0);
    let columns = visible_columns(cursor, groups.len(), width, label_width);
    rows.iter()
        .map(|midi_row| {
            let cells = groups[columns.clone()].iter().map(|group| {
                match ((midi_row.on)(group), midi_row.pitch) {
                    (true, true) => (NOTE_MARK, base_style().fg(MONOKAI_CYAN)),
                    (true, false) => (RULE_ON_MARK, base_style().fg(MONOKAI_GREEN)),
                    (false, true) => (NO_NOTE_MARK, base_style().fg(MONOKAI_GRAY)),
                    (false, false) => (" ", base_style()),
                }
            });
            row(
                &midi_row.label,
                midi_row.color,
                cells,
                label_width,
                cursor,
                columns.start,
            )
        })
        .collect()
}
