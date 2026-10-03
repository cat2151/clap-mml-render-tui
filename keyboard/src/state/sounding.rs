use std::time::Instant;

use super::PlaybackNote;

/// `t` の再生でいま鳴っている位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundingPosition {
    /// `repeat_chords()` の中の和音の index。
    pub chord_index: usize,
    /// arp のときだけ、その和音の arp 列の中の位置と音。
    pub arp: Option<ArpStep>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArpStep {
    pub index: usize,
    pub note: PlaybackNote,
}

/// 位置の変化を、音が実際に鳴る時刻つきで覚える。
///
/// tick は deadline より先に進む（予約送信のため）ので、state の現在位置は実音より先行する。
/// 描画は deadline を過ぎるまで直前の位置を見せる。
#[derive(Debug, Default)]
pub(super) struct SoundingTimeline {
    latest: Option<(Instant, SoundingPosition)>,
    previous: Option<SoundingPosition>,
}

impl SoundingTimeline {
    pub(super) fn record(&mut self, at: Instant, position: SoundingPosition) {
        self.previous = self.at(at);
        self.latest = Some((at, position));
    }

    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(super) fn at(&self, now: Instant) -> Option<SoundingPosition> {
        match self.latest {
            Some((at, position)) if at <= now => Some(position),
            Some(_) => self.previous,
            None => None,
        }
    }
}

#[cfg(test)]
mod tests;
