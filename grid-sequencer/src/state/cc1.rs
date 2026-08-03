//! 小節単位の CC1 抽選と、note on に同期する送信・表示。

use std::{collections::VecDeque, time::Instant};

use rand::Rng;

use super::{
    note_off, note_on, GridState, SoundingNote, StepDuration, CHORD_ROW, GRID_STEPS, VELOCITY,
};

const CONTROL_CHANGE: u8 = 0xB0;
const MODULATION_CC: u8 = 1;
const MODULATION_MAX: u8 = 127;

pub(crate) type Cc1DisplayRow = [Option<u8>; GRID_STEPS];
type Cc1ValueRow = [u8; GRID_STEPS];

#[derive(Debug)]
pub(super) struct Cc1State {
    /// 現在スケジュール中の小節で使う値。非発音セルも先に抽選しておく。
    values: Vec<Cc1ValueRow>,
    /// 実際に鳴っている小節の表示。`None` は CC1 を送らないステップ。
    display: Vec<Cc1DisplayRow>,
    /// 先読み済みだが、まだ小節頭が来ていない表示。
    pending_display: VecDeque<(Instant, Vec<Cc1DisplayRow>)>,
}

impl Cc1State {
    pub(super) fn new(row_count: usize) -> Self {
        Self {
            values: vec![[0; GRID_STEPS]; row_count],
            display: vec![[None; GRID_STEPS]; row_count],
            pending_display: VecDeque::new(),
        }
    }

    pub(super) fn reset_for_start(&mut self) {
        self.display.fill([None; GRID_STEPS]);
        self.pending_display.clear();
    }
}

impl GridState {
    /// 実発音中の小節に対応する CC1 grid。`None` のセルでは CC1 を送らない。
    pub(crate) fn cc1_display(&self) -> &[Cc1DisplayRow] {
        &self.cc1.display
    }

    /// 新しい小節で使う全セルの値を抽選し、送信対象だけを表示待ちへ積む。
    pub(super) fn prepare_cc1_measure(&mut self, deadline: Instant) {
        let mut rng = rand::thread_rng();
        self.draw_cc1_measure(deadline, &mut rng);
    }

    fn draw_cc1_measure(&mut self, deadline: Instant, rng: &mut impl Rng) {
        for row in &mut self.cc1.values {
            for value in row {
                *value = if rng.gen_bool(0.5) { MODULATION_MAX } else { 0 };
            }
        }
        let display = self.cc1_display_for_current_pattern();
        self.cc1.pending_display.push_back((deadline, display));
    }

    fn cc1_display_for_current_pattern(&self) -> Vec<Cc1DisplayRow> {
        self.cc1
            .values
            .iter()
            .enumerate()
            .map(|(row, values)| {
                std::array::from_fn(|step| self.row_triggers_at(row, step).then_some(values[step]))
            })
            .collect()
    }

    /// 小節途中で譜面だけが変わったとき、抽選値を保ったまま送信対象の印だけ更新する。
    pub(super) fn refresh_cc1_display_pattern(&mut self) {
        self.cc1.display = self.cc1_display_for_current_pattern();
    }

    pub(super) fn advance_cc1_display(&mut self, now: Instant) {
        while self
            .cc1
            .pending_display
            .front()
            .is_some_and(|(deadline, _)| *deadline <= now)
        {
            let (_, display) = self
                .cc1
                .pending_display
                .pop_front()
                .expect("front was just observed");
            self.cc1.display = display;
        }
    }

    /// 組み立て中の列で note on が立っている行を、CC1 を先頭にして発音する。
    pub(super) fn attack_current_step(&mut self) -> Vec<(u8, [u8; 3])> {
        let attacks = self.collect_attacks();
        let mut messages = Vec::new();
        for (row, instance_id, notes, steps) in attacks {
            messages.push((
                instance_id,
                control_change(self.cc1.values[row][self.schedule_index]),
            ));

            // 同じ instance で鳴っている音はすべて止めてから鳴らし直す。1音とは
            // 限らない（chord mode の和音）ので、1件だけ差し替えてはいけない。
            let mut released = Vec::new();
            self.sounding.retain(|note| {
                if note.instance_id == instance_id {
                    released.push(note.midi_note);
                    false
                } else {
                    true
                }
            });
            messages.extend(
                released
                    .into_iter()
                    .map(|midi_note| (instance_id, note_off(midi_note))),
            );
            for midi_note in notes {
                self.sounding.push(SoundingNote {
                    instance_id,
                    midi_note,
                    remaining_steps: steps,
                });
                messages.push((instance_id, note_on(midi_note, VELOCITY)));
            }
        }
        messages
    }

    /// この列で鳴らす (row, instance, note number 群, 持続ステップ数) を行順に集める。
    fn collect_attacks(&self) -> Vec<(usize, u8, Vec<u8>, u8)> {
        let step = self.schedule_index;
        let chord = self.chord.as_ref().map(|chord| chord.current());
        let mut attacks = Vec::new();
        for (index, row) in self.rows.iter().enumerate() {
            let instance_id = self.instance_id(index);
            match chord {
                // chord 行は小節頭に CC1 を1件送り、その後で全構成音を鳴らす。
                Some(notes) if index == CHORD_ROW => {
                    if step == 0 && !notes.is_empty() {
                        attacks.push((
                            index,
                            instance_id,
                            notes.to_vec(),
                            StepDuration::Whole.steps(),
                        ));
                    }
                }
                _ if row.cells[step] => {
                    attacks.push((index, instance_id, vec![row.note], row.duration.steps()))
                }
                _ => {}
            }
        }
        attacks
    }

    fn row_triggers_at(&self, row: usize, step: usize) -> bool {
        match self.chord.as_ref() {
            Some(chord) if row == CHORD_ROW => step == 0 && !chord.current().is_empty(),
            _ => self.rows[row].cells[step],
        }
    }
}

fn control_change(value: u8) -> [u8; 3] {
    [CONTROL_CHANGE, MODULATION_CC, value]
}

#[cfg(test)]
mod tests;
