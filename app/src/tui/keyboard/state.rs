use std::time::{Duration, Instant};

use crossterm::event::KeyCode;

use super::{KeyboardPatchCatalog, NavigationCount, NumericInput, NumericInputTarget};
use crate::history::{KeyboardSessionState, KeyboardTransport};
use crate::random::RandomIndexDeck;
use crate::realtime_play::PatchVoicing;

mod arp;
mod periodic;

pub(in crate::tui) const KEYBOARD_NOTES: [KeyboardNote; 7] = [
    KeyboardNote::new('c', "C4", 60),
    KeyboardNote::new('d', "D4", 62),
    KeyboardNote::new('e', "E4", 64),
    KeyboardNote::new('f', "F4", 65),
    KeyboardNote::new('g', "G4", 67),
    KeyboardNote::new('a', "A4", 69),
    KeyboardNote::new('b', "B4", 71),
];

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
const CONTROL_CHANGE: u8 = 0xB0;
const MODULATION_CC: u8 = 1;
const DEFAULT_VELOCITY: u8 = 100;
const ACCENT_VELOCITY: u8 = 127;
const DEFAULT_CC_NUMBER: u8 = MODULATION_CC;
const MODULATION_MAX: u8 = 127;
const CC_MAX: u8 = 127;
const PITCH_BEND: u8 = 0xE0;
// 14bit値: center=8192, +8191=16383, -8192=0
const PITCH_BEND_CENTER: u16 = 8192;
const PITCH_BEND_MAX: u16 = 16383;
const PITCH_BEND_MIN: u16 = 0;
// pitch bend周期mode(4値循環)の送信順
const PITCH_BEND_CYCLE: [u16; 4] = [
    PITCH_BEND_MAX,
    PITCH_BEND_CENTER,
    PITCH_BEND_MIN,
    PITCH_BEND_CENTER,
];
const PERIODIC_INTERVAL: Duration = Duration::from_millis(250);
const REPEAT_TICKS: u8 = 8;
// 周期mode中の桁値シーケンス。index 0は周期突入時の初回即送信値と一致させる
// (pitch bendはPITCH_BEND_CYCLEをそのまま桁値列として使う)
const VELOCITY_SEQ: [u8; 2] = [DEFAULT_VELOCITY, ACCENT_VELOCITY];
const MODULATION_SEQ: [u8; 2] = [0, MODULATION_MAX];
const CC_SEQ: [u8; 2] = [CC_MAX, 0];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(in crate::tui) enum VelocityMode {
    #[default]
    Normal,
    Accent,
    Periodic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(in crate::tui) enum ModulationMode {
    #[default]
    Off,
    On,
    Periodic,
}

// Idleは「一度もpを押していない」初期状態。
// サイクルはIdle→Max→CenterAfterMax→Min→CenterAfterMin→Periodic→CenterAfterCycle→Max→…
// (+8191, 0, -8192, 0, 4値循環, 0 の6段。Idleには戻らない)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(in crate::tui) enum PitchBendMode {
    #[default]
    Idle,
    Max,
    CenterAfterMax,
    Min,
    CenterAfterMin,
    Periodic,
    CenterAfterCycle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(in crate::tui) enum NotePlaybackMode {
    #[default]
    Off,
    Repeat,
    Arp,
    Auto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) struct KeyboardNote {
    pub(in crate::tui) key: char,
    pub(in crate::tui) name: &'static str,
    pub(in crate::tui) midi_note: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) struct PlaybackNote {
    pub(in crate::tui) midi_note: u8,
}

impl From<KeyboardNote> for PlaybackNote {
    fn from(note: KeyboardNote) -> Self {
        Self {
            midi_note: note.midi_note,
        }
    }
}

impl KeyboardNote {
    const fn new(key: char, name: &'static str, midi_note: u8) -> Self {
        Self {
            key,
            name,
            midi_note,
        }
    }
}

pub(crate) struct KeyboardState {
    held: Vec<KeyboardNote>,
    pub(super) patch: Option<String>,
    transport: KeyboardTransport,
    buffer_multiplier: u8,
    velocity: u8,
    velocity_mode: VelocityMode,
    modulation_mode: ModulationMode,
    pitch_bend_mode: PitchBendMode,
    cc_number: u8,
    cc_periodic_on: bool,
    note_playback_mode: NotePlaybackMode,
    detected_voicing: PatchVoicing,
    // repeat/arp対象のコード進行。手鍵盤では最後に押した和音を1要素として保持する
    repeat_chords: Vec<Vec<PlaybackNote>>,
    // repeat/arpで現在再生しているコード進行上の位置
    repeat_chord_index: usize,
    // note repeatで現在発音中のノート
    repeat_sounding: Vec<PlaybackNote>,
    // arpで現在発音中のノートと、次に発音するシーケンス位置
    arp_sounding: Option<PlaybackNote>,
    arp_next_index: usize,
    // 全周期系統(桁、repeat、arp)が共有する250msマスタークロック
    periodic_next_at: Option<Instant>,
    // repeatが現在の和音を鳴らしてから経過したマスタークロックのtick数
    repeat_elapsed_ticks: u8,
    // 周期modeがONの系統を桁とみなした組み合わせIDの山札(bag)。桁構成変更で作り直す
    combo_bag: Option<RandomIndexDeck>,
    // 山札から引いた現在の組み合わせID(混合基数値)。各系統の現在値はここから桁分解で導出する
    current_combo: usize,
    // patch切替完了(Ready復帰)時にコントローラ現在値を再送するフラグ
    refresh_pending: bool,
    numeric_input: Option<NumericInput>,
    pub(in crate::tui) navigation_count: NavigationCount,
    pub(in crate::tui) patch_catalog: KeyboardPatchCatalog,
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self::new(None)
    }
}

impl KeyboardState {
    pub(super) fn new(patch: Option<String>) -> Self {
        Self::from_session(KeyboardSessionState {
            patch,
            ..KeyboardSessionState::default()
        })
    }

    pub(in crate::tui) fn from_session(session: KeyboardSessionState) -> Self {
        Self {
            held: Vec::new(),
            patch: session
                .patch
                .and_then(|patch| (!patch.trim().is_empty()).then_some(patch)),
            transport: session.transport,
            buffer_multiplier: session.buffer_multiplier,
            velocity: DEFAULT_VELOCITY,
            velocity_mode: VelocityMode::default(),
            modulation_mode: ModulationMode::default(),
            pitch_bend_mode: PitchBendMode::default(),
            cc_number: DEFAULT_CC_NUMBER,
            cc_periodic_on: false,
            note_playback_mode: NotePlaybackMode::Off,
            detected_voicing: PatchVoicing::Unknown,
            repeat_chords: Vec::new(),
            repeat_chord_index: 0,
            repeat_sounding: Vec::new(),
            arp_sounding: None,
            arp_next_index: 0,
            periodic_next_at: None,
            repeat_elapsed_ticks: 0,
            combo_bag: None,
            current_combo: 0,
            refresh_pending: false,
            numeric_input: None,
            navigation_count: NavigationCount::default(),
            patch_catalog: KeyboardPatchCatalog::default(),
        }
    }

    pub(in crate::tui) fn session_state(&self) -> KeyboardSessionState {
        KeyboardSessionState {
            patch: self.patch.clone(),
            transport: self.transport,
            buffer_multiplier: self.buffer_multiplier,
        }
    }

    pub(in crate::tui) fn held(&self) -> &[KeyboardNote] {
        &self.held
    }

    pub(in crate::tui) fn patch(&self) -> Option<&str> {
        self.patch.as_deref()
    }

    pub(in crate::tui) fn buffer_multiplier(&self) -> u8 {
        self.buffer_multiplier
    }

    pub(in crate::tui) fn transport(&self) -> KeyboardTransport {
        self.transport
    }

    pub(in crate::tui) fn velocity(&self) -> u8 {
        self.velocity
    }

    pub(in crate::tui) fn velocity_mode(&self) -> VelocityMode {
        self.velocity_mode
    }

    pub(in crate::tui) fn modulation_mode(&self) -> ModulationMode {
        self.modulation_mode
    }

    pub(in crate::tui) fn pitch_bend_mode(&self) -> PitchBendMode {
        self.pitch_bend_mode
    }

    pub(in crate::tui) fn cc_periodic_on(&self) -> bool {
        self.cc_periodic_on
    }

    pub(in crate::tui) fn note_playback_mode(&self) -> NotePlaybackMode {
        self.note_playback_mode
    }

    pub(in crate::tui) fn set_detected_voicing(&mut self, voicing: PatchVoicing) {
        self.detected_voicing = voicing;
    }

    pub(in crate::tui) fn note_playback_uses_arp(&self) -> bool {
        self.note_playback_mode == NotePlaybackMode::Arp
            || (self.note_playback_mode == NotePlaybackMode::Auto
                && self.detected_voicing == PatchVoicing::Mono)
    }

    pub(in crate::tui) fn repeat_chords(&self) -> &[Vec<PlaybackNote>] {
        &self.repeat_chords
    }

    pub(in crate::tui) fn cc_number(&self) -> u8 {
        self.cc_number
    }

    pub(in crate::tui) fn numeric_input(&self) -> Option<&NumericInput> {
        self.numeric_input.as_ref()
    }

    pub(in crate::tui) fn begin_numeric_input(&mut self, target: NumericInputTarget) {
        self.numeric_input = Some(NumericInput::new(target));
    }

    pub(in crate::tui) fn numeric_input_push(&mut self, digit: char) {
        if let Some(input) = &mut self.numeric_input {
            input.push(digit);
        }
    }

    pub(super) fn numeric_input_backspace(&mut self) {
        if let Some(input) = &mut self.numeric_input {
            input.backspace();
        }
    }

    pub(super) fn cancel_numeric_input(&mut self) {
        self.numeric_input = None;
    }

    pub(super) fn confirm_numeric_input(&mut self) -> Option<[u8; 3]> {
        let input = self.numeric_input.take()?;
        let value = input.confirmed_value()?;
        match input.target() {
            NumericInputTarget::CcNumber => {
                self.cc_number = value;
                None
            }
            NumericInputTarget::CcValue => Some([CONTROL_CHANGE, self.cc_number, value]),
        }
    }

    pub(super) fn toggle_transport(&mut self) -> KeyboardTransport {
        self.transport = self.transport.toggled();
        self.transport
    }

    pub(super) fn cycle_buffer_multiplier(&mut self) -> u8 {
        self.buffer_multiplier = match self.buffer_multiplier {
            1 => 2,
            2 => 4,
            4 => 8,
            _ => 1,
        };
        self.buffer_multiplier
    }

    pub(in crate::tui) fn press(&mut self, note: KeyboardNote) -> Option<Vec<[u8; 3]>> {
        if self.held.iter().any(|held| held.key == note.key) {
            return None;
        }
        // 単独押しなら新しい和音の開始、他ノート押下中なら和音へ追加
        if self.held.is_empty() {
            self.repeat_chords.clear();
            self.repeat_chords.push(Vec::new());
            self.repeat_chord_index = 0;
        }
        let chord = self
            .repeat_chords
            .last_mut()
            .expect("a manual chord is created on its first key press");
        if !chord.iter().any(|held| held.midi_note == note.midi_note) {
            chord.push(note.into());
            // arp中の和音追加も、次tickでは完成時点の和音の最低音から始める
            self.arp_next_index = 0;
        }
        self.held.push(note);
        Some(vec![note_on(note.midi_note, self.velocity)])
    }

    pub(super) fn release(&mut self, note: KeyboardNote) -> Option<Vec<[u8; 3]>> {
        let index = self.held.iter().position(|held| held.key == note.key)?;
        self.held.remove(index);
        Some(vec![note_off(note.midi_note)])
    }

    // patch切替用: 発音中ノートのoffのみ送出し、自動送信系のmodeは維持する。
    // Ready復帰時にtake_pending_refresh_messagesがコントローラ現在値を再送する。
    pub(super) fn take_note_off_messages(&mut self) -> Vec<[u8; 3]> {
        let mut messages: Vec<[u8; 3]> = self
            .held
            .drain(..)
            .map(|note| note_off(note.midi_note))
            .collect();
        messages.extend(
            self.repeat_sounding
                .drain(..)
                .map(|note| note_off(note.midi_note)),
        );
        messages.extend(
            self.arp_sounding
                .take()
                .map(|note| note_off(note.midi_note)),
        );
        self.refresh_pending = true;
        messages
    }

    pub(in crate::tui) fn take_reset_messages(&mut self) -> Vec<[u8; 3]> {
        let mut messages: Vec<[u8; 3]> = self
            .held
            .drain(..)
            .map(|note| note_off(note.midi_note))
            .collect();
        self.note_playback_mode = NotePlaybackMode::Off;
        messages.extend(
            self.repeat_sounding
                .drain(..)
                .map(|note| note_off(note.midi_note)),
        );
        messages.extend(
            self.arp_sounding
                .take()
                .map(|note| note_off(note.midi_note)),
        );
        self.reset_progression_position();
        if self.velocity_mode == VelocityMode::Periodic {
            // 周期を止め、現在値に対応する固定modeへ降格(velocityは送信対象外)
            self.velocity_mode = if self.velocity == ACCENT_VELOCITY {
                VelocityMode::Accent
            } else {
                VelocityMode::Normal
            };
        }
        if self.modulation_mode != ModulationMode::Off {
            self.modulation_mode = ModulationMode::Off;
            messages.push([CONTROL_CHANGE, MODULATION_CC, 0]);
        }
        if self.pitch_bend_mode != PitchBendMode::Idle {
            self.pitch_bend_mode = PitchBendMode::Idle;
            messages.push(pitch_bend(PITCH_BEND_CENTER));
        }
        if std::mem::take(&mut self.cc_periodic_on) {
            messages.push([CONTROL_CHANGE, self.cc_number, 0]);
        }
        self.periodic_next_at = None;
        self.repeat_elapsed_ticks = 0;
        self.combo_bag = None;
        self.current_combo = 0;
        self.refresh_pending = false;
        messages
    }
}

pub(super) fn note_for_key(code: KeyCode) -> Option<KeyboardNote> {
    let KeyCode::Char(key) = code else {
        return None;
    };
    KEYBOARD_NOTES.iter().find(|note| note.key == key).copied()
}

fn note_on(midi_note: u8, velocity: u8) -> [u8; 3] {
    [NOTE_ON, midi_note, velocity]
}

fn note_off(midi_note: u8) -> [u8; 3] {
    [NOTE_OFF, midi_note, 0]
}

fn pitch_bend(value: u16) -> [u8; 3] {
    [
        PITCH_BEND,
        (value & 0x7F) as u8,
        ((value >> 7) & 0x7F) as u8,
    ]
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
