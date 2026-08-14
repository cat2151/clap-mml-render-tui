//! drum 行の step セル上の mouse wheel で、リズム型を差し替える。
//!
//! wheel は型（[`DrumPattern`]）を1つずつ送るだけ。固定型は同じ型へ戻れば同じ譜面が出る。
//! percussion の Random だけは、アルペジオの Random と同じく適用するたびに引き直す。
//!
//! 型は名前の並んだ list なので、送りは [`ListDirection`] に従って下で次、上で前。
//! list は行の役割ごとに分かれていて、kick 行を回しても hi-hat の型は出てこない。
//!
//! アルペジオ・ベースラインと違い、chord mode の on/off に関わらず成立する
//! （drum 行の音高はコードから導出しないため）。

use cmrt_rhythm::{DrumPattern, DrumRole};

use crate::{log_line, GridSequencerScreen, ListDirection};

impl GridSequencerScreen {
    /// drum 行の step セル上の wheel。型を1つ送ってリズムを引き直す。
    pub(crate) fn cycle_drum_pattern(
        &mut self,
        instance: usize,
        role: DrumRole,
        direction: ListDirection,
    ) {
        let pattern = self.advance_drum_pattern(instance, role, direction);
        // 引いた結果がたまたま同じ譜面でも、カーソルと表示は送った型に合わせる。
        self.last_drum = Some(pattern);
        let snapshot = self.capture_undo();
        if self.state.apply_drum_pattern(instance, pattern) {
            self.begin_manual_edit(crate::CycleRandomItem::Drum);
            self.commit_undo(snapshot);
        }
        log_line(&format!(
            "grid-sequencer: drum instance={instance} role={} pattern={}",
            role.label(),
            pattern.label()
        ));
    }

    /// instance ごとのカーソルを1つ送って、次に適用する型を返す。
    ///
    /// カーソルはリストの手前にある扱いで始める。初回の down は役割の list の先頭、
    /// 初回の up は末尾になる。役割が変わったカーソルは信じずに引き直す。
    fn advance_drum_pattern(
        &mut self,
        instance: usize,
        role: DrumRole,
        direction: ListDirection,
    ) -> DrumPattern {
        let current = self
            .drum_patterns
            .get(&instance)
            .copied()
            .filter(|pattern: &DrumPattern| pattern.role() == role);
        let next = match (current, direction) {
            (Some(current), ListDirection::Next) => current.next(),
            (Some(current), ListDirection::Prev) => current.previous(),
            (None, ListDirection::Next) => DrumPattern::default_for(role),
            (None, ListDirection::Prev) => DrumPattern::default_for(role).previous(),
        };
        self.drum_patterns.insert(instance, next);
        next
    }

    /// instance 番号の意味が変わるとき（`t` キーの track 数切替）にカーソルを捨てる。
    pub(crate) fn reset_drum_patterns(&mut self) {
        self.drum_patterns.clear();
        self.last_drum = None;
    }

    /// 直近に適用したリズム型。NOTE grid のタイトルと右 pane に出す。
    pub fn last_drum(&self) -> Option<DrumPattern> {
        self.last_drum
    }

    /// roleごとの直近の抽選・手動適用結果。右paneの各sectionの印に使う。
    pub(crate) fn last_drum_for(&self, role: DrumRole) -> Option<DrumPattern> {
        self.drum_patterns
            .values()
            .copied()
            .find(|pattern| pattern.role() == role)
    }
}

#[cfg(test)]
mod tests;
