//! アルペジエーターの設定と、その overlay（`z`）のキー。MML 欄の素材は書き換えず、
//! 設定を当てて鳴らす列だけを作り直す。

use std::ops::RangeInclusive;

use crossterm::event::{KeyCode, KeyEvent};

use crate::ui::ARP_PATTERN_KEYS;
use crate::{ArpSettings, ARP_CYCLES, ARP_TURN};

use super::{GuitarArticulationAction, GuitarArticulationScreen, Take};

impl GuitarArticulationScreen {
    /// MML 欄に当てて鳴らすアルペジエーターの設定。
    pub fn arp(&self) -> &ArpSettings {
        &self.arp
    }

    /// 設定を差し替え、MML の確定と同じく列ルールを付け替えて作り直し、履歴へ積んで Articulated を鳴らす。
    /// 設定が変わらなければ何もしない。素材の MML を解釈できなければ設定を戻し、理由を `error` に出す。
    pub fn set_arp(&mut self, arp: ArpSettings) -> GuitarArticulationAction {
        if arp == self.arp {
            return GuitarArticulationAction::Continue;
        }
        let before = std::mem::replace(&mut self.arp, arp);
        let mml = self.mml.clone();
        if let Err(reason) = self.commit_mml(&mml) {
            self.arp = before;
            self.error = Some(reason);
            return GuitarArticulationAction::Continue;
        }
        self.play(Take::Converted)
    }

    /// ON の間、repeat で隙間なく繋げる 1 周の秒。最初の音から最後の音を離すまでで、
    /// 音型の step 数 × 1 step になる。OFF か音が無ければ `None`。
    pub fn arp_loop_seconds(&self) -> Option<f64> {
        if !self.arp.enabled {
            return None;
        }
        let on = self.notes.first()?.on_seconds;
        let off = self
            .notes
            .iter()
            .map(|note| note.off_seconds)
            .fold(f64::MIN, f64::max);
        Some(off - on).filter(|seconds| *seconds > 0.0)
    }

    /// アルペジエーター overlay（`z`）を開いているか。
    pub fn arp_overlay_open(&self) -> bool {
        self.arp_overlay
    }

    /// overlay を開き、開いている間の repeat で Articulated を繰り返し鳴らす。MML が空なら開くだけ。
    pub(super) fn open_arp_overlay(&mut self) -> GuitarArticulationAction {
        self.arp_overlay = true;
        if self.plain.is_empty() {
            return GuitarArticulationAction::Continue;
        }
        self.play(Take::Converted)
    }

    /// overlay を閉じる。`Shift+R` が OFF なら、開いている間の repeat を止める。
    fn close_arp_overlay(&mut self) -> GuitarArticulationAction {
        self.arp_overlay = false;
        if self.repeat {
            GuitarArticulationAction::Continue
        } else {
            GuitarArticulationAction::StopRepeat
        }
    }

    /// overlay 中のキー。設定を変えるキーは作り直して鳴らし、変わらなければ何もしない。
    /// 音型・オクターブのキーは OFF なら ON にする。`0` は他の値を保ったまま OFF にする。
    pub(super) fn handle_arp_overlay_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let arp = self.arp;
        let next = match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('z') => return self.close_arp_overlay(),
            KeyCode::Char(' ') => return self.play(Take::Converted),
            KeyCode::Char('0') => ArpSettings {
                enabled: false,
                ..arp
            },
            KeyCode::Char(ch @ '1'..='3') => ArpSettings {
                enabled: true,
                octaves: usize::from(ch as u8 - b'0'),
                ..arp
            },
            KeyCode::Char('n') => ArpSettings {
                cycles: stepped(arp.cycles, 1, ARP_CYCLES),
                ..arp
            },
            KeyCode::Char('N') => ArpSettings {
                cycles: stepped(arp.cycles, -1, ARP_CYCLES),
                ..arp
            },
            KeyCode::Char('b') => ArpSettings {
                turn: stepped(arp.turn, 1, ARP_TURN),
                ..arp
            },
            KeyCode::Char('B') => ArpSettings {
                turn: stepped(arp.turn, -1, ARP_TURN),
                ..arp
            },
            KeyCode::Char(ch) => match ARP_PATTERN_KEYS.iter().find(|(key, _)| *key == ch) {
                Some(&(_, pattern)) => ArpSettings {
                    enabled: true,
                    pattern,
                    ..arp
                },
                None => return GuitarArticulationAction::Continue,
            },
            _ => return GuitarArticulationAction::Continue,
        };
        self.set_arp(next)
    }
}

/// `value` を `delta` だけ動かし、範囲の端で止める。
fn stepped(value: usize, delta: isize, range: RangeInclusive<usize>) -> usize {
    value
        .saturating_add_signed(delta)
        .clamp(*range.start(), *range.end())
}

#[cfg(test)]
mod tests;
