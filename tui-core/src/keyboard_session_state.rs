//! 起動・終了で保存・復元する keyboard 画面のセッション状態。

const DEFAULT_KEYBOARD_BUFFER_MULTIPLIER: u8 = 4;
const DEFAULT_CC_NUMBER: u8 = 1;
const CC_MAX: u8 = 127;

/// keyboard 画面の `t`（note 再生モード）。
///
/// `Auto` は判定した音色が mono なら arp、それ以外なら repeat として振る舞う。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotePlaybackMode {
    #[default]
    Off,
    Repeat,
    Arp,
    Auto,
}

/// keyboard 画面の `v`（velocity）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VelocityMode {
    #[default]
    Normal,
    Accent,
    Periodic,
}

/// keyboard 画面の `m`（modulation）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModulationMode {
    #[default]
    Off,
    On,
    Periodic,
}

/// keyboard 画面の `p`（pitch bend）。
///
/// `Idle` は「一度も `p` を押していない」初期状態。巡回は
/// Max→CenterAfterMax→Min→CenterAfterMin→Periodic→CenterAfterCycle→Max→… で、Idle には戻らない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PitchBendMode {
    #[default]
    Idle,
    Max,
    CenterAfterMax,
    Min,
    CenterAfterMin,
    Periodic,
    CenterAfterCycle,
}

/// keyboard 画面のコントローラ（`v` `m` `p` `x` `Z`）の選択。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct KeyboardControllerState {
    pub velocity: VelocityMode,
    pub modulation: ModulationMode,
    pub pitch_bend: PitchBendMode,
    /// `x` で選んだ CC 番号。`Z` の周期送信先。
    pub cc_number: u8,
    /// `Z` の周期送信が on か。
    pub cc_periodic: bool,
}

impl Default for KeyboardControllerState {
    fn default() -> Self {
        Self {
            velocity: VelocityMode::Normal,
            modulation: ModulationMode::Off,
            pitch_bend: PitchBendMode::Idle,
            cc_number: DEFAULT_CC_NUMBER,
            cc_periodic: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeyboardSessionState {
    #[serde(default)]
    pub patch: Option<String>,
    #[serde(default = "default_keyboard_buffer_multiplier")]
    pub buffer_multiplier: u8,
    #[serde(default)]
    pub note_playback_mode: NotePlaybackMode,
    /// repeat / arp の対象。MIDI ノート番号の和音の列。
    #[serde(default)]
    pub repeat_chords: Vec<Vec<u8>>,
    /// `i` で最後に確定した MML。復元では解釈し直さない（対象は `repeat_chords` が持つ）。
    #[serde(default)]
    pub mml: String,
    /// 鳴っている音へ掛ける effect chain。段は catalog が返した JSON 値のまま。
    #[serde(default)]
    pub effect_chain: Vec<serde_json::Value>,
    /// Patches pane の絞り込み条件。plugin solo/mute もこの条件の plugin term として入る。
    #[serde(default)]
    pub patch_filter: String,
    #[serde(default)]
    pub controllers: KeyboardControllerState,
}

impl Default for KeyboardSessionState {
    fn default() -> Self {
        Self {
            patch: None,
            buffer_multiplier: DEFAULT_KEYBOARD_BUFFER_MULTIPLIER,
            note_playback_mode: NotePlaybackMode::Off,
            repeat_chords: Vec::new(),
            mml: String::new(),
            effect_chain: Vec::new(),
            patch_filter: String::new(),
            controllers: KeyboardControllerState::default(),
        }
    }
}

const fn default_keyboard_buffer_multiplier() -> u8 {
    DEFAULT_KEYBOARD_BUFFER_MULTIPLIER
}

impl KeyboardSessionState {
    pub fn normalize(&mut self) {
        self.patch = self
            .patch
            .take()
            .and_then(|patch| (!patch.trim().is_empty()).then_some(patch));
        if !matches!(self.buffer_multiplier, 1 | 2 | 4 | 8) {
            self.buffer_multiplier = DEFAULT_KEYBOARD_BUFFER_MULTIPLIER;
        }
        if self.controllers.cc_number > CC_MAX {
            self.controllers.cc_number = DEFAULT_CC_NUMBER;
        }
    }
}

#[cfg(test)]
mod tests;
