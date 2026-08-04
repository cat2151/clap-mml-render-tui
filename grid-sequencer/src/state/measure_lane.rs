//! 小節単位でパターンを抽選し、実発音まで表示を待たせる汎用レーン。
//!
//! CC1 modulation と note velocity は「小節頭に全セルの値を決め、note on する
//! セルだけ送信・表示する」点がまったく同じなので、候補値だけを差し替えて
//! 同じ仕組みを使い回す。値の決め方そのものは [`pattern`] にある。

use std::{collections::VecDeque, time::Instant};

use rand::Rng;

use super::{GridState, GRID_STEPS};

pub(super) mod pattern;

use pattern::{ramp_value, MeasurePattern, MeasurePlan};

/// 1行ぶんの表示。`None` は「そのステップでは発音しない＝値を送らない」。
pub(crate) type LaneDisplayRow = [Option<u8>; GRID_STEPS];
/// 行×ステップの発音表。どのセルを抽選値の送信対象にするかを決める。
pub(super) type TriggerTable = Vec<[bool; GRID_STEPS]>;

type LaneValueRow = [u8; GRID_STEPS];

/// 実発音1小節ぶんの表示。パターン名も値の表示と同じタイミングで切り替えたいので、
/// 先読みのキューには2つまとめて積む。
#[derive(Debug)]
struct LaneMeasure {
    rows: Vec<LaneDisplayRow>,
    /// 再生前とリセット直後は `None`。まだ鳴っている小節がない。
    pattern: Option<MeasurePattern>,
}

#[derive(Debug)]
pub(super) struct MeasureLane {
    /// このレーンが取りうる `[低い値, 高い値]`。
    choices: [u8; 2],
    /// 現在スケジュール中の小節で使う値。非発音セルも先に決めておく。
    values: Vec<LaneValueRow>,
    /// 実際に鳴っている小節の表示。
    display: LaneMeasure,
    /// 先読み済みだが、まだ小節頭が来ていない表示。
    pending_display: VecDeque<(Instant, LaneMeasure)>,
}

impl MeasureLane {
    pub(super) fn new(row_count: usize, choices: [u8; 2]) -> Self {
        Self {
            choices,
            values: vec![[choices[0]; GRID_STEPS]; row_count],
            display: LaneMeasure {
                rows: vec![[None; GRID_STEPS]; row_count],
                pattern: None,
            },
            pending_display: VecDeque::new(),
        }
    }

    pub(super) fn reset_for_start(&mut self) {
        self.display.rows.fill([None; GRID_STEPS]);
        self.display.pattern = None;
        self.pending_display.clear();
    }

    pub(super) fn display(&self) -> &[LaneDisplayRow] {
        &self.display.rows
    }

    /// 実発音中の小節のパターン。表示と同じく、先読みぶんは先出ししない。
    pub(super) fn pattern(&self) -> Option<MeasurePattern> {
        self.display.pattern
    }

    /// 実発音のときに送る値。表示ではなく、スケジュール中の小節の抽選結果を見る。
    pub(super) fn value_at(&self, row: usize, step: usize) -> u8 {
        self.values[row][step]
    }

    /// 新しい小節で使う全セルの値を `pattern` に従って決め、送信対象だけを
    /// 表示待ちへ積む。
    pub(super) fn draw_measure(
        &mut self,
        deadline: Instant,
        pattern: MeasurePattern,
        rng: &mut impl Rng,
        triggers: &[[bool; GRID_STEPS]],
    ) {
        match pattern {
            MeasurePattern::Random => self.fill_random(rng),
            MeasurePattern::Ramp { ascending } => self.fill_ramp(ascending),
        }
        let rows = self.display_for(triggers);
        self.pending_display.push_back((
            deadline,
            LaneMeasure {
                rows,
                pattern: Some(pattern),
            },
        ));
    }

    /// 小節途中で譜面だけが変わったとき、決まった値を保ったまま送信対象の印だけ更新する。
    pub(super) fn refresh_display(&mut self, triggers: &[[bool; GRID_STEPS]]) {
        self.display.rows = self.display_for(triggers);
    }

    pub(super) fn advance_display(&mut self, now: Instant) {
        while self
            .pending_display
            .front()
            .is_some_and(|(deadline, _)| *deadline <= now)
        {
            let (_, display) = self
                .pending_display
                .pop_front()
                .expect("front was just observed");
            self.display = display;
        }
    }

    fn fill_random(&mut self, rng: &mut impl Rng) {
        let choices = self.choices;
        for row in &mut self.values {
            for value in row {
                *value = choices[usize::from(rng.gen_bool(0.5))];
            }
        }
    }

    /// ランプはステップ位置だけで決まるので、全行が同じ並びになる。
    fn fill_ramp(&mut self, ascending: bool) {
        let choices = self.choices;
        let row: LaneValueRow = std::array::from_fn(|step| ramp_value(choices, ascending, step));
        self.values.fill(row);
    }

    fn display_for(&self, triggers: &[[bool; GRID_STEPS]]) -> Vec<LaneDisplayRow> {
        self.values
            .iter()
            .zip(triggers)
            .map(|(values, triggers)| {
                std::array::from_fn(|step| triggers[step].then_some(values[step]))
            })
            .collect()
    }
}

impl GridState {
    /// 新しい小節へ入るとき、全レーンの値を抽選し直す。
    pub(super) fn prepare_lane_measures(&mut self, deadline: Instant) {
        let mut rng = rand::thread_rng();
        self.draw_lane_measures(deadline, &mut rng);
    }

    /// 小節途中で譜面だけが変わったとき、全レーンの送信対象を引き直す。
    pub(super) fn refresh_lane_display_patterns(&mut self) {
        // 発音表を値として先に作る。レーンへ `&self` を借りるクロージャを渡すと、
        // レーン自身の可変借用と衝突してコンパイルが通らない。
        let triggers = self.trigger_table();
        self.cc1.refresh_display(&triggers);
        self.velocity.refresh_display(&triggers);
    }

    pub(super) fn advance_lane_displays(&mut self, now: Instant) {
        self.cc1.advance_display(now);
        self.velocity.advance_display(now);
    }

    pub(super) fn reset_lanes_for_start(&mut self) {
        self.cc1.reset_for_start();
        self.velocity.reset_for_start();
    }

    fn draw_lane_measures(&mut self, deadline: Instant, rng: &mut impl Rng) {
        let triggers = self.trigger_table();
        // パターンは小節につき1回だけ引く。CC1 と velocity は同じ抽選結果を分け合う。
        let plan = MeasurePlan::draw(rng);
        self.cc1.draw_measure(deadline, plan.cc1, rng, &triggers);
        self.velocity
            .draw_measure(deadline, plan.velocity, rng, &triggers);
    }
}

#[cfg(test)]
mod tests;
