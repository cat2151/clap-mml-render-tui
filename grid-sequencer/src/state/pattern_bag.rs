//! random対象のフレーズ型をまとめて引くshuffle bag。
//!
//! テトリスの7-bagと同じく、有効な対象の全組み合わせをshuffleした袋から1つずつ引き、
//! 空になったら補充する。Cycle Randomの対象別に袋を分けるため、DRUMだけなら6通り、
//! ARP（bass + arpeggio）だけなら54通り、両方なら324通りを重複なしで1周する。

use std::collections::HashMap;

use cmrt_arpeggiator::{ArpPattern, BassPattern};
use cmrt_rhythm::{DrumPattern, DrumPatternCombination, DrumRole};
use cmrt_tui_core::random::RandomIndexDeck;

/// 1回の抽選で同時に適用するフレーズ型の組み合わせ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatternCombination {
    drums: Option<DrumPatternCombination>,
    bass: Option<BassPattern>,
    arp: Option<ArpPattern>,
}

impl PatternCombination {
    fn all(include_drums: bool, include_arp: bool) -> Vec<Self> {
        if !include_drums && !include_arp {
            return Vec::new();
        }
        let drums = if include_drums {
            DrumPatternCombination::all()
                .into_iter()
                .map(Some)
                .collect::<Vec<_>>()
        } else {
            vec![None]
        };
        let basses = if include_arp {
            BassPattern::ALL.into_iter().map(Some).collect::<Vec<_>>()
        } else {
            vec![None]
        };
        let arpeggios = if include_arp {
            ArpPattern::ALL.into_iter().map(Some).collect::<Vec<_>>()
        } else {
            vec![None]
        };

        let mut combinations = Vec::new();
        for drums in &drums {
            for bass in &basses {
                for arp in &arpeggios {
                    combinations.push(Self {
                        drums: *drums,
                        bass: *bass,
                        arp: *arp,
                    });
                }
            }
        }
        combinations
    }

    pub fn drum_pattern(self, role: DrumRole) -> Option<DrumPattern> {
        self.drums.map(|drums| drums.pattern_for(role))
    }

    pub fn bass_pattern(self) -> Option<BassPattern> {
        self.bass
    }

    pub fn arp_pattern(self) -> Option<ArpPattern> {
        self.arp
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PatternBagKey {
    drums: bool,
    arp: bool,
}

#[derive(Debug)]
struct PatternBag {
    combinations: Vec<PatternCombination>,
    deck: RandomIndexDeck,
}

impl PatternBag {
    fn new(key: PatternBagKey) -> Self {
        let combinations = PatternCombination::all(key.drums, key.arp);
        let deck = RandomIndexDeck::new(combinations.len());
        Self { combinations, deck }
    }

    fn draw(&mut self) -> PatternCombination {
        if self.deck.is_exhausted() {
            self.deck = RandomIndexDeck::new(self.combinations.len());
        }
        self.combinations[self.deck.next_index()]
    }
}

#[derive(Debug, Default)]
pub(super) struct PatternBags {
    bags: HashMap<PatternBagKey, PatternBag>,
}

impl PatternBags {
    pub(super) fn draw(
        &mut self,
        include_drums: bool,
        include_arp: bool,
    ) -> Option<PatternCombination> {
        let key = PatternBagKey {
            drums: include_drums,
            arp: include_arp,
        };
        if !key.drums && !key.arp {
            return None;
        }
        Some(
            self.bags
                .entry(key)
                .or_insert_with(|| PatternBag::new(key))
                .draw(),
        )
    }
}

#[cfg(test)]
mod tests;
