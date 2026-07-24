//! 起動・終了で保存・復元する keyboard 画面のセッション状態。
//!
//! 永続化そのもの（ファイル入出力）は `cmrt-history` が担う。ここはデータ形と
//! 正規化ルールだけを持つ値ドメインで、keyboard 画面（`cmrt-keyboard`）と
//! 永続化層の双方から参照されるため `cmrt-tui-core` に置いている。

const DEFAULT_KEYBOARD_BUFFER_MULTIPLIER: u8 = 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardTransport {
    Http,
    #[default]
    SharedMemory,
}

impl KeyboardTransport {
    pub fn label(self) -> &'static str {
        match self {
            Self::Http => "HTTP",
            Self::SharedMemory => "SHM",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Http => Self::SharedMemory,
            Self::SharedMemory => Self::Http,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeyboardSessionState {
    #[serde(default)]
    pub patch: Option<String>,
    #[serde(default)]
    pub transport: KeyboardTransport,
    #[serde(default = "default_keyboard_buffer_multiplier")]
    pub buffer_multiplier: u8,
}

impl Default for KeyboardSessionState {
    fn default() -> Self {
        Self {
            patch: None,
            transport: KeyboardTransport::SharedMemory,
            buffer_multiplier: DEFAULT_KEYBOARD_BUFFER_MULTIPLIER,
        }
    }
}

const fn default_keyboard_buffer_multiplier() -> u8 {
    DEFAULT_KEYBOARD_BUFFER_MULTIPLIER
}

impl KeyboardSessionState {
    /// 読み込んだ値の不正を既定値へ丸める。
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
