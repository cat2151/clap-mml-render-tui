//! 1行ぶんの note event pattern。
//!
//! `Tie` は同じ小節内の直前の `Attack` だけを延長する。外部へ配列の可変参照を
//! 渡さず、編集後も orphan Tie が残らないことをこの型で保証する。

use super::GRID_STEPS;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NoteStep {
    #[default]
    Rest,
    Attack,
    Tie,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotePattern {
    steps: [NoteStep; GRID_STEPS],
}

impl Default for NotePattern {
    fn default() -> Self {
        Self {
            steps: [NoteStep::Rest; GRID_STEPS],
        }
    }
}

impl NotePattern {
    /// 任意長の入力を16 stepへ丸め、orphan TieをRestへ正規化する。
    pub fn from_steps(steps: impl IntoIterator<Item = NoteStep>) -> Self {
        let mut pattern = Self::default();
        for (target, step) in pattern.steps.iter_mut().zip(steps) {
            *target = step;
        }
        pattern.normalize();
        pattern
    }

    pub fn step(&self, step: usize) -> Option<NoteStep> {
        self.steps.get(step).copied()
    }

    pub fn steps(&self) -> &[NoteStep; GRID_STEPS] {
        &self.steps
    }

    /// `start..=end`へ1つのnoteを描く。終端は小節末尾へclampする。
    pub fn draw_span(&mut self, start: usize, end: usize) -> bool {
        if start >= GRID_STEPS {
            return false;
        }
        let end = end.min(GRID_STEPS - 1);
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let before = self.clone();

        // span内に開始点を持つ既存noteは、そのtailも含めて先に除去する。
        // これにより、置換したAttackの古いTieが新spanの終端を越えて残らない。
        for step in start..=end {
            if before.is_attack(step) {
                let _ = self.erase_note_at(step);
            }
        }
        // startが既存noteのTieでも、独立した新しいAttackを置けばそこでretriggerする。
        self.steps[start] = NoteStep::Attack;
        for step in start + 1..=end {
            self.steps[step] = NoteStep::Tie;
        }
        self.normalize();
        *self != before
    }

    /// 指定cellが属するnote eventをAttackと全Tieごと消す。
    pub fn erase_note_at(&mut self, step: usize) -> bool {
        let Some(kind) = self.step(step) else {
            return false;
        };
        if kind == NoteStep::Rest {
            return false;
        }
        let mut attack = step;
        while self.steps[attack] == NoteStep::Tie {
            if attack == 0 {
                // 正規化済みなら到達しないが、境界では安全にno-opにする。
                return false;
            }
            attack -= 1;
        }
        if self.steps[attack] != NoteStep::Attack {
            return false;
        }
        self.steps[attack] = NoteStep::Rest;
        let mut tail = attack + 1;
        while tail < GRID_STEPS && self.steps[tail] == NoteStep::Tie {
            self.steps[tail] = NoteStep::Rest;
            tail += 1;
        }
        true
    }

    pub fn clear(&mut self) -> bool {
        if self.steps.iter().all(|step| *step == NoteStep::Rest) {
            return false;
        }
        self.steps.fill(NoteStep::Rest);
        true
    }

    pub fn attack_len(&self, step: usize) -> Option<u8> {
        if !self.is_attack(step) {
            return None;
        }
        let tail = self.steps[step + 1..]
            .iter()
            .take_while(|kind| **kind == NoteStep::Tie)
            .count();
        Some((tail + 1) as u8)
    }

    pub fn is_attack(&self, step: usize) -> bool {
        self.step(step) == Some(NoteStep::Attack)
    }

    pub fn sounding_end(&self) -> Option<usize> {
        self.steps.iter().rposition(|step| *step != NoteStep::Rest)
    }

    fn normalize(&mut self) {
        let mut sounding = false;
        for step in &mut self.steps {
            match *step {
                NoteStep::Rest => sounding = false,
                NoteStep::Attack => sounding = true,
                NoteStep::Tie if !sounding => *step = NoteStep::Rest,
                NoteStep::Tie => {}
            }
        }
    }
}

#[cfg(test)]
mod tests;
