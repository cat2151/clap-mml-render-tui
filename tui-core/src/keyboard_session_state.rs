//! 起動・終了で保存・復元する keyboard 画面のセッション状態。

const DEFAULT_KEYBOARD_BUFFER_MULTIPLIER: u8 = 4;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeyboardSessionState {
    #[serde(default)]
    pub patch: Option<String>,
    #[serde(default = "default_keyboard_buffer_multiplier")]
    pub buffer_multiplier: u8,
}

impl Default for KeyboardSessionState {
    fn default() -> Self {
        Self {
            patch: None,
            buffer_multiplier: DEFAULT_KEYBOARD_BUFFER_MULTIPLIER,
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
    }
}
