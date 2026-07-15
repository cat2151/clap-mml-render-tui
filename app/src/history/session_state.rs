use anyhow::Result;

const DEFAULT_KEYBOARD_BUFFER_MULTIPLIER: u8 = 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardTransport {
    Http,
    #[default]
    SharedMemory,
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
    fn normalize(&mut self) {
        self.patch = self
            .patch
            .take()
            .and_then(|patch| (!patch.trim().is_empty()).then_some(patch));
        if !matches!(self.buffer_multiplier, 1 | 2 | 4 | 8) {
            self.buffer_multiplier = DEFAULT_KEYBOARD_BUFFER_MULTIPLIER;
        }
    }
}

/// 起動・終了で保存・復元するセッション状態。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionState {
    /// 現在行番号（0始まり）。
    #[serde(default)]
    pub cursor: usize,
    /// 編集行リスト。
    #[serde(default = "super::helpers::default_lines")]
    pub lines: Vec<String>,
    /// 終了時に DAW モードだったかどうか。起動時に復元する。
    #[serde(default)]
    pub is_daw_mode: bool,
    /// keyboard 画面で終了した場合に、次回復帰する状態。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyboard: Option<KeyboardSessionState>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            cursor: 0,
            lines: super::helpers::default_lines(),
            is_daw_mode: false,
            keyboard: None,
        }
    }
}

/// セッション状態（現在行番号）を history.json に保存する。
/// データディレクトリが利用できない場合はベストエフォートでスキップする。
pub fn save_session_state(state: &SessionState) -> Result<()> {
    let _ = super::paths::migrate_legacy_history_file("history.json");
    let Some(path) = super::paths::session_state_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// history.json からセッション状態を読み込む。
/// ファイルが存在しない場合・データディレクトリが利用できない場合・読み込みに失敗した場合は
/// デフォルト値を返す。
/// `lines` が空の場合（`"lines": []` のような入力）はデフォルト値で補填し、
/// `lines` が常に1行以上という不変条件を保証する。
pub fn load_session_state() -> SessionState {
    let Some(path) = super::paths::resolved_history_file_path("history.json") else {
        return SessionState::default();
    };
    if !path.exists() {
        return SessionState::default();
    }
    let mut state: SessionState = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if state.lines.is_empty() {
        state.lines = super::helpers::default_lines();
    }
    if let Some(keyboard) = &mut state.keyboard {
        keyboard.normalize();
        state.is_daw_mode = false;
    }
    state
}
