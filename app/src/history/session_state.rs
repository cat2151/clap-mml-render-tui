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
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionState {
    /// 現在行番号（0始まり）。
    #[serde(default)]
    pub cursor: usize,
    /// 編集行リスト。
    #[serde(default = "super::helpers::default_lines")]
    pub lines: Vec<String>,
    /// 終了時に表示していた主要画面。起動時に直接復元する。
    pub active_screen: crate::screen_switch::PrimaryScreen,
    /// 最後に使用した keyboard 状態。表示画面とは独立して保持する。
    pub keyboard: KeyboardSessionState,
    /// keyboard の音出し確認 overlay を最後に表示したローカル日付（YYYY-MM-DD）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyboard_note_guide_overlay_date: Option<String>,
    /// notepad の音出し確認 overlay を最後に表示したローカル日付（YYYY-MM-DD）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notepad_sound_check_guide_overlay_date: Option<String>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            cursor: 0,
            lines: super::helpers::default_lines(),
            active_screen: crate::screen_switch::PrimaryScreen::Notepad,
            keyboard: KeyboardSessionState::default(),
            keyboard_note_guide_overlay_date: None,
            notepad_sound_check_guide_overlay_date: None,
        }
    }
}

#[derive(serde::Deserialize)]
struct SessionStateWire {
    #[serde(default)]
    cursor: usize,
    #[serde(default = "super::helpers::default_lines")]
    lines: Vec<String>,
    #[serde(default)]
    active_screen: Option<crate::screen_switch::PrimaryScreen>,
    #[serde(default)]
    is_daw_mode: bool,
    #[serde(default)]
    keyboard: Option<KeyboardSessionState>,
    #[serde(default)]
    keyboard_note_guide_overlay_date: Option<String>,
    #[serde(default)]
    notepad_sound_check_guide_overlay_date: Option<String>,
}

impl<'de> serde::Deserialize<'de> for SessionState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = SessionStateWire::deserialize(deserializer)?;
        let active_screen = wire.active_screen.unwrap_or_else(|| {
            if wire.keyboard.is_some() {
                crate::screen_switch::PrimaryScreen::Keyboard
            } else if wire.is_daw_mode {
                crate::screen_switch::PrimaryScreen::Daw
            } else {
                crate::screen_switch::PrimaryScreen::Notepad
            }
        });
        Ok(Self {
            cursor: wire.cursor,
            lines: wire.lines,
            active_screen,
            keyboard: wire.keyboard.unwrap_or_default(),
            keyboard_note_guide_overlay_date: wire.keyboard_note_guide_overlay_date,
            notepad_sound_check_guide_overlay_date: wire.notepad_sound_check_guide_overlay_date,
        })
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

/// keyboard の音出し確認 overlay の表示日だけを即時保存する。
///
/// 実行中の未保存編集や、復元用 keyboard 状態を意図せず上書きしないよう、
/// ディスク上のセッション状態へ日付だけをマージする。
pub(crate) fn save_keyboard_note_guide_overlay_date(local_date: &str) -> Result<()> {
    let mut state = load_session_state();
    state.keyboard_note_guide_overlay_date = Some(local_date.to_owned());
    save_session_state(&state)
}

/// notepad の音出し確認 overlay の表示日だけを即時保存する。
pub(crate) fn save_notepad_sound_check_guide_overlay_date(local_date: &str) -> Result<()> {
    let mut state = load_session_state();
    state.notepad_sound_check_guide_overlay_date = Some(local_date.to_owned());
    save_session_state(&state)
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
    state.keyboard.normalize();
    state
}
