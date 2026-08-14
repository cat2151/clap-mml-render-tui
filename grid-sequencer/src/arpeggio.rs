//! step セル上の mouse wheel で、instance の譜面をランダムなアルペジオへ差し替える。
//!
//! wheel は音型（[`ArpPattern`]）を1つずつ送るだけで、duration は毎回引き直す。
//! 同じ音型を回し続けても違うフレーズが出るので、聴き比べながら詰められる。
//!
//! 音型は名前の並んだ list なので、送りは [`ListDirection`] に従って下で次、上で前。
//!
//! 音高は現在のコード構成音から導出されるため chord mode 中しか成立しない。声部が
//! 2つ未満の instance（Single lane や和音の行）は対象外として黙って無視する。

use cmrt_arpeggiator::{generate_arpeggio, ArpPattern};

use crate::{log_line, DrawnPhrases, GridSequencerScreen, ListDirection, GRID_STEPS};

impl GridSequencerScreen {
    /// step セル上の wheel。音型を1つ送ってアルペジオを引き直す。
    pub(crate) fn cycle_arpeggio(&mut self, instance: usize, direction: ListDirection) {
        let voice_count = self.state.arp_voice_count(instance);
        if voice_count < 2 {
            return;
        }
        let pattern = self.advance_arp_pattern(instance, direction);
        // 引いた結果がたまたま同じ譜面でも、カーソルと表示は送った音型に合わせる。
        self.state
            .display_drawn_now(DrawnPhrases::with_arp(pattern));
        let notes = generate_arpeggio(pattern, voice_count, GRID_STEPS, &mut rand::rng());
        let snapshot = self.capture_undo();
        if !self.state.apply_arpeggio(instance, &notes) {
            return;
        }
        self.begin_manual_edit(crate::CycleRandomItem::Arp);
        self.commit_undo(snapshot);
        log_line(&format!(
            "grid-sequencer: arpeggio instance={instance} pattern={} voices={voice_count}",
            pattern.label()
        ));
    }

    /// instance ごとのカーソルを1つ送って、次に適用する音型を返す。
    ///
    /// カーソルはリストの手前にある扱いで始める。初回の down は先頭（`Up`）、初回の
    /// up は末尾（`Random`）になる。
    fn advance_arp_pattern(&mut self, instance: usize, direction: ListDirection) -> ArpPattern {
        let next = match (self.arp_patterns.get(&instance).copied(), direction) {
            (Some(current), ListDirection::Next) => current.next(),
            (Some(current), ListDirection::Prev) => current.previous(),
            (None, ListDirection::Next) => ArpPattern::default(),
            (None, ListDirection::Prev) => ArpPattern::default().previous(),
        };
        self.arp_patterns.insert(instance, next);
        next
    }

    /// instance 番号の意味が変わるとき（`t` キーの track 数切替）にカーソルを捨てる。
    pub(crate) fn reset_arp_patterns(&mut self) {
        self.arp_patterns.clear();
        self.state.clear_displayed_arp();
    }

    /// 直近に適用した音型。NOTE grid のタイトルへ出す。
    pub fn last_arp(&self) -> Option<ArpPattern> {
        self.state.displayed_drawn_phrases().arp
    }
}

#[cfg(test)]
mod tests;
