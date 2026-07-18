use super::*;

// 周期modeがONの系統ごとの桁値index。current_comboの混合基数分解で決まる
#[derive(Clone, Copy, Default)]
struct ComboDigits {
    velocity: usize,
    modulation: usize,
    pitch_bend: usize,
    cc: usize,
}

impl KeyboardState {
    pub(in crate::tui) fn cycle_velocity(&mut self, now: Instant) -> VelocityMode {
        match self.velocity_mode {
            VelocityMode::Normal => {
                self.velocity = ACCENT_VELOCITY;
                self.velocity_mode = VelocityMode::Accent;
            }
            VelocityMode::Accent => {
                // 周期突入で即反転(127→100=VELOCITY_SEQ先頭)し、以降は毎tickのbag値になる
                self.velocity = DEFAULT_VELOCITY;
                self.velocity_mode = VelocityMode::Periodic;
                self.reset_combo_clock(now);
            }
            VelocityMode::Periodic => {
                self.velocity = DEFAULT_VELOCITY;
                self.velocity_mode = VelocityMode::Normal;
                self.reset_combo_clock(now);
            }
        }
        self.velocity_mode
    }

    pub(in crate::tui) fn cycle_modulation(&mut self, now: Instant) -> [u8; 3] {
        let value = match self.modulation_mode {
            ModulationMode::Off => {
                self.modulation_mode = ModulationMode::On;
                MODULATION_MAX
            }
            ModulationMode::On => {
                // 直前がON=127なので周期の初回は0(MODULATION_SEQ先頭)から
                self.modulation_mode = ModulationMode::Periodic;
                self.reset_combo_clock(now);
                0
            }
            ModulationMode::Periodic => {
                self.modulation_mode = ModulationMode::Off;
                self.reset_combo_clock(now);
                0
            }
        };
        [CONTROL_CHANGE, MODULATION_CC, value]
    }

    pub(in crate::tui) fn cycle_pitch_bend(&mut self, now: Instant) -> [u8; 3] {
        let value = match self.pitch_bend_mode {
            PitchBendMode::Idle | PitchBendMode::CenterAfterCycle => {
                self.pitch_bend_mode = PitchBendMode::Max;
                PITCH_BEND_MAX
            }
            PitchBendMode::Max => {
                self.pitch_bend_mode = PitchBendMode::CenterAfterMax;
                PITCH_BEND_CENTER
            }
            PitchBendMode::CenterAfterMax => {
                self.pitch_bend_mode = PitchBendMode::Min;
                PITCH_BEND_MIN
            }
            PitchBendMode::Min => {
                self.pitch_bend_mode = PitchBendMode::CenterAfterMin;
                PITCH_BEND_CENTER
            }
            PitchBendMode::CenterAfterMin => {
                // 周期の初回は+8191(PITCH_BEND_CYCLE先頭)を即送信し、以降は毎tickのbag値
                self.pitch_bend_mode = PitchBendMode::Periodic;
                self.reset_combo_clock(now);
                PITCH_BEND_MAX
            }
            PitchBendMode::Periodic => {
                self.pitch_bend_mode = PitchBendMode::CenterAfterCycle;
                self.reset_combo_clock(now);
                PITCH_BEND_CENTER
            }
        };
        pitch_bend(value)
    }

    pub(in crate::tui) fn toggle_cc_periodic(&mut self, now: Instant) -> [u8; 3] {
        self.cc_periodic_on = !self.cc_periodic_on;
        // OFF状態は実質0なので周期の初回は127(CC_SEQ先頭)から
        let value = if self.cc_periodic_on { CC_MAX } else { 0 };
        self.reset_combo_clock(now);
        [CONTROL_CHANGE, self.cc_number, value]
    }

    pub(super) fn attack_repeat_chord(&mut self) -> Vec<[u8; 3]> {
        self.repeat_sounding = self.repeat_chord.clone();
        let velocity = self.velocity;
        self.repeat_chord
            .iter()
            .map(|note| note_on(*note, velocity))
            .collect()
    }

    pub(in crate::tui) fn poll_periodic(&mut self, now: Instant) -> Vec<[u8; 3]> {
        if !deadline_elapsed(&mut self.periodic_next_at, now) {
            return Vec::new();
        }
        self.draw_next_combo();
        let digits = self.combo_digits();
        let mut messages = Vec::new();
        if self.velocity_mode == VelocityMode::Periodic {
            // velocityはメッセージを持たず、直後のnote onに反映される
            self.velocity = VELOCITY_SEQ[digits.velocity];
        }
        if self.modulation_mode == ModulationMode::Periodic {
            messages.push([
                CONTROL_CHANGE,
                MODULATION_CC,
                MODULATION_SEQ[digits.modulation],
            ]);
        }
        if self.pitch_bend_mode == PitchBendMode::Periodic {
            messages.push(pitch_bend(PITCH_BEND_CYCLE[digits.pitch_bend]));
        }
        if self.cc_periodic_on {
            messages.push([CONTROL_CHANGE, self.cc_number, CC_SEQ[digits.cc]]);
        }
        match self.note_playback_mode {
            NotePlaybackMode::Off => {}
            NotePlaybackMode::Repeat => {
                // リトリガーは値変更の後: 直前のoffと現在の和音のonを送る
                messages.extend(self.repeat_sounding.drain(..).map(note_off));
                messages.extend(self.attack_repeat_chord());
            }
            NotePlaybackMode::Arp => messages.extend(self.advance_arp()),
        }
        messages
    }

    // patch切替完了(Ready復帰)後に、自動送信系の現在値を新patchへ再送する
    pub(in crate::tui) fn take_pending_refresh_messages(&mut self, now: Instant) -> Vec<[u8; 3]> {
        if !std::mem::take(&mut self.refresh_pending) {
            return Vec::new();
        }
        let digits = self.combo_digits();
        let mut messages = Vec::new();
        match self.modulation_mode {
            ModulationMode::Off => {}
            ModulationMode::On => messages.push([CONTROL_CHANGE, MODULATION_CC, MODULATION_MAX]),
            ModulationMode::Periodic => messages.push([
                CONTROL_CHANGE,
                MODULATION_CC,
                MODULATION_SEQ[digits.modulation],
            ]),
        }
        let pitch_bend_value = match self.pitch_bend_mode {
            PitchBendMode::Idle => None,
            PitchBendMode::Max => Some(PITCH_BEND_MAX),
            PitchBendMode::Min => Some(PITCH_BEND_MIN),
            PitchBendMode::CenterAfterMax
            | PitchBendMode::CenterAfterMin
            | PitchBendMode::CenterAfterCycle => Some(PITCH_BEND_CENTER),
            PitchBendMode::Periodic => Some(PITCH_BEND_CYCLE[digits.pitch_bend]),
        };
        if let Some(value) = pitch_bend_value {
            messages.push(pitch_bend(value));
        }
        if self.cc_periodic_on {
            messages.push([CONTROL_CHANGE, self.cc_number, CC_SEQ[digits.cc]]);
        }
        match self.note_playback_mode {
            NotePlaybackMode::Off => {}
            NotePlaybackMode::Repeat => {
                self.periodic_next_at = Some(now + PERIODIC_INTERVAL);
                messages.extend(self.attack_repeat_chord());
            }
            NotePlaybackMode::Arp => messages.extend(self.restart_arp(now)),
        }
        messages
    }

    // 周期桁が1つ以上ONのとき(bag消化数, 組み合わせ総数)を返す(UI表示用)
    pub(in crate::tui) fn combo_progress(&self) -> Option<(usize, usize)> {
        if !self.periodic_digits_active() {
            return None;
        }
        let drawn = self
            .combo_bag
            .as_ref()
            .map_or(0, RandomIndexDeck::drawn_count);
        Some((drawn, self.combo_total()))
    }

    pub(super) fn periodic_digits_active(&self) -> bool {
        self.velocity_mode == VelocityMode::Periodic
            || self.modulation_mode == ModulationMode::Periodic
            || self.pitch_bend_mode == PitchBendMode::Periodic
            || self.cc_periodic_on
    }

    fn periodic_active(&self) -> bool {
        self.periodic_digits_active() || self.note_playback_mode != NotePlaybackMode::Off
    }

    // 桁構成が変わるトグルで呼ぶ。bagを破棄して組み合わせ先頭へ戻し、クロックを再スタートする
    fn reset_combo_clock(&mut self, now: Instant) {
        self.combo_bag = None;
        self.current_combo = 0;
        if self.velocity_mode == VelocityMode::Periodic {
            self.velocity = VELOCITY_SEQ[0];
        }
        self.periodic_next_at = self.periodic_active().then(|| now + PERIODIC_INTERVAL);
    }

    // tick毎に山札から次の組み合わせIDを引く。山札が空/桁構成変更後は再シャッフルして作り直す
    fn draw_next_combo(&mut self) {
        let total = self.combo_total();
        if total <= 1 {
            self.combo_bag = None;
            self.current_combo = 0;
            return;
        }
        let needs_rebuild = self
            .combo_bag
            .as_ref()
            .is_none_or(|bag| bag.total() != total || bag.is_exhausted());
        if needs_rebuild {
            self.combo_bag = Some(RandomIndexDeck::new(total));
        }
        if let Some(bag) = &mut self.combo_bag {
            self.current_combo = bag.next_index();
        }
    }

    fn combo_total(&self) -> usize {
        let mut total = 1;
        if self.pitch_bend_mode == PitchBendMode::Periodic {
            total *= PITCH_BEND_CYCLE.len();
        }
        if self.modulation_mode == ModulationMode::Periodic {
            total *= MODULATION_SEQ.len();
        }
        if self.cc_periodic_on {
            total *= CC_SEQ.len();
        }
        if self.velocity_mode == VelocityMode::Periodic {
            total *= VELOCITY_SEQ.len();
        }
        total
    }

    // current_comboを最下位桁から分解する。桁順はPB→mod→CC→velocityで固定
    fn combo_digits(&self) -> ComboDigits {
        let mut rest = self.current_combo;
        let mut digits = ComboDigits::default();
        if self.pitch_bend_mode == PitchBendMode::Periodic {
            digits.pitch_bend = rest % PITCH_BEND_CYCLE.len();
            rest /= PITCH_BEND_CYCLE.len();
        }
        if self.modulation_mode == ModulationMode::Periodic {
            digits.modulation = rest % MODULATION_SEQ.len();
            rest /= MODULATION_SEQ.len();
        }
        if self.cc_periodic_on {
            digits.cc = rest % CC_SEQ.len();
            rest /= CC_SEQ.len();
        }
        if self.velocity_mode == VelocityMode::Periodic {
            digits.velocity = rest % VELOCITY_SEQ.len();
        }
        digits
    }
}

// deadline到達なら次回を1周期後へ進めてtrue。大幅遅延時(非Ready停滞など)は
// now基準へスナップし、復帰直後のバースト送信を防ぐ(欠落サイクルはスキップ)。
fn deadline_elapsed(next_at: &mut Option<Instant>, now: Instant) -> bool {
    let Some(deadline) = *next_at else {
        return false;
    };
    if now < deadline {
        return false;
    }
    let scheduled = deadline + PERIODIC_INTERVAL;
    *next_at = Some(if scheduled <= now {
        now + PERIODIC_INTERVAL
    } else {
        scheduled
    });
    true
}

#[cfg(test)]
#[path = "periodic_tests.rs"]
mod tests;
