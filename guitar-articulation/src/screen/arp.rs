//! アルペジエーターの設定と、その overlay（`z`）のキー。
//!
//! overlay は素材（[`GuitarArticulationScreen::arp_material`]）と設定を自分で持ち、開いている間だけ
//! それで鳴らす列を作る。列ごとのルールは当てない。MML 欄・列ごとのルール・列ルールの付け替え元・
//! カーソル列には触らず、閉じたら MML 欄の MML とルール表で鳴らす列を作り直す。
//! 行全体のルールとアクセント（[`super::arp_rules`]）はメイン画面と共有し、履歴へは閉じたときに開く前と違えば 1 件だけ積む。
//!
//! overlay は param pane と素材 pane の 2 つ。param pane は `j` / `k` で行を選び、`h` / `l` で値を動かす。
//! 素材 pane（[`super::arp_materials`]）は 1 行 1 素材の入力欄。

use std::ops::RangeInclusive;

use cmrt_arpeggiator::ArpPattern;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui_textarea::TextArea;

use crate::{ArpSettings, RuleTable, ARP_BPM, ARP_DOWN, ARP_OCTAVES, ARP_PATTERNS, ARP_SHIFT};

use super::{GuitarArticulationAction, GuitarArticulationScreen, Take};

/// param pane の行。並びは表示の並び。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArpRow {
    Material,
    Pattern,
    Octaves,
    Shift,
    /// [`ArpPattern::UpDown`] のときだけ出る。
    Down,
    /// chord 素材のときだけ出る。
    Bpm,
    /// chord 素材のときだけ出る。
    Rate,
    /// 汚し（メイン画面と共有）。
    Humanize,
    /// 奏法（メイン画面と共有）。
    Picking,
    /// アクセント（メイン画面と共有）。
    Accent,
}

const ALL_ROWS: [ArpRow; 10] = [
    ArpRow::Material,
    ArpRow::Pattern,
    ArpRow::Octaves,
    ArpRow::Shift,
    ArpRow::Down,
    ArpRow::Bpm,
    ArpRow::Rate,
    ArpRow::Humanize,
    ArpRow::Picking,
    ArpRow::Accent,
];

/// overlay を開いている間の状態。
pub(super) struct ArpOverlay {
    /// param pane で選んでいる行。今は出ない行なら、それより前で出ている行を選んでいるとみなす。
    row: ArpRow,
    /// 素材 pane を編集している間だけ `Some`。
    pub(super) materials: Option<TextArea<'static>>,
    help_open: bool,
    /// 開く直前のルール表。閉じるとき、行全体のルールが変わったかをこれと比べる。
    rules_before: RuleTable,
}

impl GuitarArticulationScreen {
    /// アルペジエーターの設定。閉じている間も値は保つ。
    pub fn arp(&self) -> &ArpSettings {
        &self.arp
    }

    /// 鳴らす列に当てているアルペジエーターの設定。overlay を開いている間だけ `Some`。
    pub fn applied_arp(&self) -> Option<&ArpSettings> {
        self.arp_overlay.is_some().then_some(&self.arp)
    }

    /// 保存済みのアルペジエーターの設定を持たせる。
    pub fn with_arp(mut self, arp: ArpSettings) -> Self {
        self.arp = arp;
        self
    }

    /// 設定を差し替える。overlay を開いていれば鳴らす列を作り直して Articulated を鳴らし、閉じていれば値を持つだけ。
    /// 設定が変わらなければ何もしない。素材を解釈できなければ設定を戻し、理由を `error` に出す。
    /// 変わった設定は設定 file へ書く対象になる（[`Self::take_unsaved_settings`]）。
    pub fn set_arp(&mut self, arp: ArpSettings) -> GuitarArticulationAction {
        if arp == self.arp {
            return GuitarArticulationAction::Continue;
        }
        let before = std::mem::replace(&mut self.arp, arp);
        if self.arp_overlay.is_none() {
            self.settings_unsaved = true;
            return GuitarArticulationAction::Continue;
        }
        let material = self.arp_material().to_string();
        if let Err(reason) = self.rebuild_arp_performance(&material) {
            self.arp = before;
            self.error = Some(reason);
            return GuitarArticulationAction::Continue;
        }
        self.settings_unsaved = true;
        self.play(Take::Converted)
    }

    /// overlay の素材。overlay で選んだ素材が無ければ MML 欄の MML。
    pub fn arp_material(&self) -> &str {
        if self.arp_material.is_empty() {
            &self.mml
        } else {
            &self.arp_material
        }
    }

    /// 鳴らす列の素材。overlay を開いている間は overlay の素材、閉じていれば MML 欄の MML。
    pub fn sounding_mml(&self) -> &str {
        if self.arp_overlay.is_some() {
            self.arp_material()
        } else {
            &self.mml
        }
    }

    /// 鳴らす列の素材が chord 表記として解釈されたか。BPM と音価はこのときだけ効く。
    pub fn material_from_chord(&self) -> bool {
        self.material_from_chord
    }

    /// 当てている間、repeat で隙間なく繋げる 1 周の秒。最初の音から最後の音を離すまでで、
    /// 音型の step 数 × 1 step になる。当てていないか音が無ければ `None`。
    pub fn arp_loop_seconds(&self) -> Option<f64> {
        self.applied_arp()?;
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
        self.arp_overlay.is_some()
    }

    /// overlay の中のヘルプ（`?`）を開いているか。
    pub fn arp_help_open(&self) -> bool {
        self.arp_overlay
            .as_ref()
            .is_some_and(|overlay| overlay.help_open)
    }

    /// param pane に今出ている行。下り幅は UpDown、BPM と音価は chord 素材のときだけ。
    pub fn arp_rows(&self) -> Vec<ArpRow> {
        ALL_ROWS
            .into_iter()
            .filter(|row| match row {
                ArpRow::Down => self.arp.pattern == ArpPattern::UpDown,
                ArpRow::Bpm | ArpRow::Rate => self.material_from_chord,
                _ => true,
            })
            .collect()
    }

    /// param pane で選んでいる行の、[`Self::arp_rows`] での位置。overlay を閉じていれば `None`。
    pub fn arp_row_index(&self) -> Option<usize> {
        let row = self.arp_overlay.as_ref()?.row;
        let rows = self.arp_rows();
        Some(
            rows.iter()
                .rposition(|shown| *shown <= row)
                .unwrap_or_default(),
        )
    }

    /// overlay を開いて overlay の素材と設定で鳴らす列を作り、開いている間の repeat で Articulated を繰り返し鳴らす。
    /// 前に選んだ素材を解釈できなければ理由を `error` に出し、MML 欄の MML から始める。音が無ければ開くだけ。
    pub(super) fn open_arp_overlay(&mut self) -> GuitarArticulationAction {
        self.arp_overlay = Some(ArpOverlay {
            row: ArpRow::Material,
            materials: None,
            help_open: false,
            rules_before: self.rules.clone(),
        });
        let material = self.arp_material().to_string();
        if let Err(reason) = self.rebuild_arp_performance(&material) {
            self.arp_material.clear();
            self.settings_unsaved = true;
            let mml = self.mml.clone();
            if let Err(fallback) = self.rebuild_arp_performance(&mml) {
                self.error = Some(fallback);
                return GuitarArticulationAction::Continue;
            }
            self.error = Some(reason);
        }
        if self.plain.is_empty() {
            return GuitarArticulationAction::Continue;
        }
        self.play(Take::Converted)
    }

    /// overlay を閉じ、MML 欄の MML とルール表で鳴らす列を作り直す。`Shift+R` が ON ならその演奏で repeat を続け、
    /// OFF なら開いている間の repeat を止める。行全体のルールが開く前と違えば、今の状態を履歴へ積む。
    fn close_arp_overlay(&mut self) -> GuitarArticulationAction {
        if let Some(overlay) = self.arp_overlay.take() {
            self.record_arp_rules(&overlay.rules_before);
        }
        let mml = self.mml.clone();
        if let Err(reason) = self.set_performance(&mml, None) {
            self.error = Some(reason);
        }
        self.rebuild_converted();
        if !self.repeat {
            GuitarArticulationAction::StopRepeat
        } else if self.plain.is_empty() {
            GuitarArticulationAction::Continue
        } else {
            self.play(Take::Converted)
        }
    }

    /// overlay を開いている間の鳴らす列を、`material` に今の設定を当てて作り直す。
    /// 解釈できなければ何も変えない。
    pub(super) fn rebuild_arp_performance(&mut self, material: &str) -> Result<(), String> {
        let arp = self.arp;
        self.set_performance(material, Some(&arp))?;
        self.rebuild_converted();
        Ok(())
    }

    /// overlay 中のキー。素材 pane の編集中は素材 pane へ、ヘルプ中は閉じるキーだけを見る。
    pub(super) fn handle_arp_overlay_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        self.error = None;
        let Some(overlay) = self.arp_overlay.as_mut() else {
            return GuitarArticulationAction::Continue;
        };
        if overlay.help_open {
            if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
                overlay.help_open = false;
            }
            return GuitarArticulationAction::Continue;
        }
        if overlay.materials.is_some() {
            return self.handle_arp_materials_key(key);
        }
        match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('z') => self.close_arp_overlay(),
            KeyCode::Char('?') => {
                overlay.help_open = true;
                GuitarArticulationAction::Continue
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.open_arp_materials();
                GuitarArticulationAction::Continue
            }
            KeyCode::Char(' ') => self.play(Take::Converted),
            KeyCode::Char('j') | KeyCode::Down => self.move_arp_row(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_arp_row(-1),
            KeyCode::Char('h') | KeyCode::Left => self.step_arp_row(-1, false),
            KeyCode::Char('l') | KeyCode::Right => self.step_arp_row(1, false),
            KeyCode::Char('H') => self.step_arp_row(-1, true),
            KeyCode::Char('L') => self.step_arp_row(1, true),
            _ => GuitarArticulationAction::Continue,
        }
    }

    /// param pane の行を `delta` だけ動かし、端で止める。
    fn move_arp_row(&mut self, delta: isize) -> GuitarArticulationAction {
        let rows = self.arp_rows();
        let Some(index) = self.arp_row_index() else {
            return GuitarArticulationAction::Continue;
        };
        let next = stepped(index, delta, 0..=rows.len() - 1);
        if let Some(overlay) = self.arp_overlay.as_mut() {
            overlay.row = rows[next];
        }
        GuitarArticulationAction::Continue
    }

    /// 選んでいる行の値を `delta` だけ動かして鳴らす。値は端で止まり、変わらなければ何もしない。
    /// `fine` は BPM を 1 ずつ動かす（他の行は同じ）。
    fn step_arp_row(&mut self, delta: isize, fine: bool) -> GuitarArticulationAction {
        let Some(index) = self.arp_row_index() else {
            return GuitarArticulationAction::Continue;
        };
        let arp = self.arp;
        let next = match self.arp_rows()[index] {
            ArpRow::Material => return self.cycle_arp_material(delta),
            ArpRow::Pattern => ArpSettings {
                pattern: stepped_pattern(arp.pattern, delta),
                ..arp
            },
            ArpRow::Octaves => ArpSettings {
                octaves: stepped(arp.octaves, delta, ARP_OCTAVES),
                ..arp
            },
            ArpRow::Shift => ArpSettings {
                shift: stepped_shift(arp.shift, delta as i8),
                ..arp
            },
            ArpRow::Down => ArpSettings {
                down: stepped_down(arp.down, delta),
                ..arp
            },
            ArpRow::Bpm => ArpSettings {
                bpm: stepped_bpm(arp.bpm, delta as i16 * if fine { 1 } else { 5 }),
                ..arp
            },
            ArpRow::Rate => ArpSettings {
                rate: arp.rate.stepped(delta),
                ..arp
            },
            ArpRow::Humanize => return self.step_arp_humanize(delta),
            ArpRow::Picking => return self.step_arp_picking(delta),
            ArpRow::Accent => return self.step_arp_accent(delta),
        };
        self.set_arp(next)
    }
}

/// `value` を `delta` だけ動かし、範囲の端で止める。
pub(super) fn stepped(value: usize, delta: isize, range: RangeInclusive<usize>) -> usize {
    value
        .saturating_add_signed(delta)
        .clamp(*range.start(), *range.end())
}

/// 音型を [`ARP_PATTERNS`] の並びで `delta` だけ動かし、端で止める。
fn stepped_pattern(pattern: ArpPattern, delta: isize) -> ArpPattern {
    let index = ARP_PATTERNS
        .iter()
        .position(|candidate| *candidate == pattern)
        .unwrap_or_default();
    ARP_PATTERNS[stepped(index, delta, 0..=ARP_PATTERNS.len() - 1)]
}

/// oct シフトを `delta` だけ動かし、[`ARP_SHIFT`] の端で止める。
fn stepped_shift(shift: i8, delta: i8) -> i8 {
    shift
        .saturating_add(delta)
        .clamp(*ARP_SHIFT.start(), *ARP_SHIFT.end())
}

/// BPM を `delta` だけ動かし、[`ARP_BPM`] の端で止める。
fn stepped_bpm(bpm: u16, delta: i16) -> u16 {
    bpm.saturating_add_signed(delta)
        .clamp(*ARP_BPM.start(), *ARP_BPM.end())
}

/// 下り幅を `[1, 2, …, 8, 全部]` の並びで `delta` だけ動かし、端で止める。
fn stepped_down(down: Option<usize>, delta: isize) -> Option<usize> {
    let all = *ARP_DOWN.end() + 1;
    let moved = stepped(down.unwrap_or(all), delta, *ARP_DOWN.start()..=all);
    (moved != all).then_some(moved)
}

#[cfg(test)]
mod tests;
