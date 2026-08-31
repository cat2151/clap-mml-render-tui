//! Grid Sequencer の選択 track と、一時的な Solo mix 状態。

use crate::{GridSequencerScreen, CHORD_ROW};

/// 完全ランダム演奏中の chord mode で、和音の行へ与える音量差（dB）。
pub const CHORD_GAIN_DB: f32 = 6.0;

/// instance ごとの音量差（dB）。返す長さは、先読み先を含む bank 2 本ぶん。
pub fn chord_gains_db(instance_count: usize, chord_on: bool, note_random: bool) -> Vec<f32> {
    let boost_chord = chord_on && note_random;
    (0..instance_count * cmrt_realtime_play::BANK_COUNT)
        .map(|instance| {
            if boost_chord && instance % instance_count == CHORD_ROW {
                CHORD_GAIN_DB
            } else {
                0.0
            }
        })
        .collect()
}

impl GridSequencerScreen {
    pub(crate) fn selected_track(&self) -> usize {
        self.selected_track
    }

    pub(crate) fn solo_mode_active(&self) -> bool {
        self.solo_tracks.iter().any(|&solo| solo)
    }

    pub(crate) fn track_is_soloed(&self, track: usize) -> bool {
        self.solo_tracks.get(track).copied().unwrap_or(false)
    }

    pub(crate) fn track_is_audible(&self, track: usize) -> bool {
        !self.solo_mode_active() || self.track_is_soloed(track)
    }

    pub(crate) fn select_track(&mut self, track: usize) {
        if track < self.track_count() {
            self.selected_track = track;
        }
    }

    pub(crate) fn select_next_track(&mut self) {
        self.selected_track = self
            .selected_track
            .saturating_add(1)
            .min(self.track_count().saturating_sub(1));
    }

    pub(crate) fn select_previous_track(&mut self) {
        self.selected_track = self.selected_track.saturating_sub(1);
    }

    pub(crate) fn toggle_selected_track_solo(&mut self) {
        self.toggle_track_solo(self.selected_track);
    }

    pub(crate) fn toggle_track_solo(&mut self, track: usize) {
        let Some(solo) = self.solo_tracks.get_mut(track) else {
            return;
        };
        *solo = !*solo;
        self.apply_playback_gains();
    }

    pub(crate) fn resize_track_mix_state(&mut self, track_count: usize) {
        self.solo_tracks.resize(track_count, false);
        self.solo_tracks.truncate(track_count);
        self.selected_track = self.selected_track.min(track_count.saturating_sub(1));
    }

    /// Solo mask と chord 行の固定 boost を合成した、両 bank の振幅倍率。
    fn playback_amplitude_gains(&self) -> Vec<f32> {
        chord_gains_db(
            self.state.instance_count(),
            self.state.chord().is_some(),
            self.cycle_random.note,
        )
        .into_iter()
        .enumerate()
        .map(|(instance, gain_db)| {
            let track = instance % self.state.instance_count();
            if self.track_is_audible(track) {
                10.0f32.powf(gain_db / 20.0)
            } else {
                0.0
            }
        })
        .collect()
    }

    /// ゲインは音色ロードをまたいでサーバーに残るため、Solo・chord boostのどちらを
    /// 変えた場合も、常に合成済みの全instance値を送り直す。
    pub(crate) fn apply_playback_gains(&self) {
        let Some(sender) = &self.midi_sender else {
            return;
        };
        sender.set_amplitude_gains(self.playback_amplitude_gains());
    }
}

#[cfg(test)]
mod tests;
