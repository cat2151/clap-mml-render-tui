use super::*;

impl KeyboardState {
    pub(in crate::tui) fn cycle_velocity(&mut self, now: Instant) -> VelocityMode {
        self.velocity_mode = match self.velocity_mode {
            VelocityMode::Normal => {
                self.velocity = ACCENT_VELOCITY;
                self.velocity_next_at = None;
                VelocityMode::Accent
            }
            VelocityMode::Accent => {
                // 周期突入で即反転(127→100)し、以降1秒ごとにpoll_periodicが反転する
                self.velocity = DEFAULT_VELOCITY;
                self.velocity_next_at = Some(now + PERIODIC_INTERVAL);
                VelocityMode::Periodic
            }
            VelocityMode::Periodic => {
                self.velocity = DEFAULT_VELOCITY;
                self.velocity_next_at = None;
                VelocityMode::Normal
            }
        };
        self.velocity_mode
    }

    pub(in crate::tui) fn cycle_modulation(&mut self, now: Instant) -> [u8; 3] {
        let value = match self.modulation_mode {
            ModulationMode::Off => {
                self.modulation_mode = ModulationMode::On;
                MODULATION_MAX
            }
            ModulationMode::On => {
                // 直前がON=127なので周期の初回は0から
                self.modulation_mode = ModulationMode::Periodic;
                self.modulation_phase_high = true;
                self.modulation_next_at = Some(now + PERIODIC_INTERVAL);
                0
            }
            ModulationMode::Periodic => {
                self.modulation_mode = ModulationMode::Off;
                self.modulation_next_at = None;
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
                // 周期の初回は+8191を即送信し、以降0→-8192→0→+8191…の4値循環
                self.pitch_bend_mode = PitchBendMode::Periodic;
                self.pitch_bend_phase = 1;
                self.pitch_bend_next_at = Some(now + PERIODIC_INTERVAL);
                PITCH_BEND_MAX
            }
            PitchBendMode::Periodic => {
                self.pitch_bend_mode = PitchBendMode::CenterAfterCycle;
                self.pitch_bend_next_at = None;
                PITCH_BEND_CENTER
            }
        };
        pitch_bend(value)
    }

    pub(in crate::tui) fn toggle_cc_periodic(&mut self, now: Instant) -> [u8; 3] {
        self.cc_periodic_on = !self.cc_periodic_on;
        let value = if self.cc_periodic_on {
            // OFF状態は実質0なので周期の初回は127から
            self.cc_phase_high = false;
            self.cc_next_at = Some(now + PERIODIC_INTERVAL);
            CC_MAX
        } else {
            self.cc_next_at = None;
            0
        };
        [CONTROL_CHANGE, self.cc_number, value]
    }

    // 最後に押した和音の1秒周期リトリガーをトグルする。和音が未確定ならOFFのまま。
    pub(in crate::tui) fn toggle_note_repeat(&mut self, now: Instant) -> Vec<[u8; 3]> {
        if self.note_repeat_on {
            self.note_repeat_on = false;
            self.note_repeat_next_at = None;
            self.repeat_sounding.drain(..).map(note_off).collect()
        } else {
            if self.repeat_chord.is_empty() {
                return Vec::new();
            }
            self.note_repeat_on = true;
            self.note_repeat_next_at = Some(now + PERIODIC_INTERVAL);
            self.attack_repeat_chord()
        }
    }

    fn attack_repeat_chord(&mut self) -> Vec<[u8; 3]> {
        self.repeat_sounding = self.repeat_chord.clone();
        let velocity = self.velocity;
        self.repeat_chord
            .iter()
            .map(|note| note_on(*note, velocity))
            .collect()
    }

    pub(in crate::tui) fn poll_periodic(&mut self, now: Instant) -> Vec<[u8; 3]> {
        let mut messages = Vec::new();
        if deadline_elapsed(&mut self.velocity_next_at, now) {
            self.velocity = if self.velocity == ACCENT_VELOCITY {
                DEFAULT_VELOCITY
            } else {
                ACCENT_VELOCITY
            };
        }
        if deadline_elapsed(&mut self.modulation_next_at, now) {
            let value = if self.modulation_phase_high {
                MODULATION_MAX
            } else {
                0
            };
            self.modulation_phase_high = !self.modulation_phase_high;
            messages.push([CONTROL_CHANGE, MODULATION_CC, value]);
        }
        if deadline_elapsed(&mut self.pitch_bend_next_at, now) {
            let value = PITCH_BEND_CYCLE[self.pitch_bend_phase];
            self.pitch_bend_phase = (self.pitch_bend_phase + 1) % PITCH_BEND_CYCLE.len();
            messages.push(pitch_bend(value));
        }
        if deadline_elapsed(&mut self.cc_next_at, now) {
            let value = if self.cc_phase_high { CC_MAX } else { 0 };
            self.cc_phase_high = !self.cc_phase_high;
            messages.push([CONTROL_CHANGE, self.cc_number, value]);
        }
        if deadline_elapsed(&mut self.note_repeat_next_at, now) {
            // 毎秒リトリガー: 直前に鳴らしたノートのoffと現在の和音のonを同時に送る
            let offs: Vec<[u8; 3]> = self.repeat_sounding.drain(..).map(note_off).collect();
            messages.extend(offs);
            messages.extend(self.attack_repeat_chord());
        }
        messages
    }

    // patch切替完了(Ready復帰)後に、自動送信系の現在値を新patchへ再送する
    pub(in crate::tui) fn take_pending_refresh_messages(&mut self) -> Vec<[u8; 3]> {
        if !std::mem::take(&mut self.refresh_pending) {
            return Vec::new();
        }
        let mut messages = Vec::new();
        match self.modulation_mode {
            ModulationMode::Off => {}
            ModulationMode::On => messages.push([CONTROL_CHANGE, MODULATION_CC, MODULATION_MAX]),
            // 周期中の現在値=「次に送る値」の逆
            ModulationMode::Periodic => messages.push([
                CONTROL_CHANGE,
                MODULATION_CC,
                if self.modulation_phase_high {
                    0
                } else {
                    MODULATION_MAX
                },
            ]),
        }
        let pitch_bend_value = match self.pitch_bend_mode {
            PitchBendMode::Idle => None,
            PitchBendMode::Max => Some(PITCH_BEND_MAX),
            PitchBendMode::Min => Some(PITCH_BEND_MIN),
            PitchBendMode::CenterAfterMax
            | PitchBendMode::CenterAfterMin
            | PitchBendMode::CenterAfterCycle => Some(PITCH_BEND_CENTER),
            PitchBendMode::Periodic => Some(
                PITCH_BEND_CYCLE
                    [(self.pitch_bend_phase + PITCH_BEND_CYCLE.len() - 1) % PITCH_BEND_CYCLE.len()],
            ),
        };
        if let Some(value) = pitch_bend_value {
            messages.push(pitch_bend(value));
        }
        if self.cc_periodic_on {
            messages.push([
                CONTROL_CHANGE,
                self.cc_number,
                if self.cc_phase_high { 0 } else { CC_MAX },
            ]);
        }
        if self.note_repeat_on {
            let attack = self.attack_repeat_chord();
            messages.extend(attack);
        }
        messages
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
