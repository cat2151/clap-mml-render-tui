//! 鳴らす列（raw・音・Articulated・汚し）の作り直し。

use std::borrow::Cow;

use crate::humanize;
use crate::{
    articulate, convert, material_performance, notes_from_events, ArpSettings, RowRule, RuleTable,
};

use super::playhead_map::PlayheadMap;
use super::{GuitarArticulationScreen, Take};

impl GuitarArticulationScreen {
    /// 素材に `arp` を当てた raw の列と音を作り、素材が chord 表記かを覚える。Articulated は作り直さない。
    /// 解釈できなければ何も変えない。
    pub(super) fn set_performance(
        &mut self,
        material: &str,
        arp: Option<&ArpSettings>,
    ) -> Result<(), String> {
        let performance = material_performance(material, arp)?;
        self.material_from_chord = performance.from_chord;
        self.notes = notes_from_events(&performance.events);
        self.plain = performance.events;
        Ok(())
    }

    /// 鳴らす列に当てるルール表。アルペジエーター overlay を開いている間は列ごとのルールを除く。
    pub fn sounding_rules(&self) -> Cow<'_, RuleTable> {
        if self.arp_overlay.is_some() {
            Cow::Owned(self.rules.without_column_rules())
        } else {
            Cow::Borrowed(&self.rules)
        }
    }

    /// Articulated・汚し・演奏位置の表（[`PlayheadMap`]）を、今の raw・音・ルールから作り直す。
    pub(super) fn rebuild_converted(&mut self) {
        let rules = self.sounding_rules();
        let articulated = articulate(&self.notes, &rules);
        let converted = convert(&self.plain, &rules);
        let humanized = if rules.is_row_on(RowRule::Humanize) {
            humanize::seeded(&self.notes, &articulated)
        } else {
            Vec::new()
        };
        self.articulated = articulated;
        self.converted = converted;
        self.humanized = humanized;
        self.playhead_map = PlayheadMap::new(
            &self.column_starts(Take::Plain),
            &self.plain,
            &self.column_starts(Take::Converted),
            &self.converted,
        );
    }
}
