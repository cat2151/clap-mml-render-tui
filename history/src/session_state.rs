use anyhow::Result;
use serde::Deserialize;

// keyboard 画面のセッション状態は `cmrt-tui-core` が所有する。
pub use cmrt_tui_core::keyboard_session_state::KeyboardSessionState;

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
    pub active_screen: cmrt_tui_core::screen_switch::PrimaryScreen,
    /// 最後に使用した keyboard 状態。表示画面とは独立して保持する。
    pub keyboard: KeyboardSessionState,
    /// Grid Sequencer が使用する track / CLAP instance 数。
    pub grid_sequencer_track_count: usize,
    /// Grid Sequencer の chord mode が on だったか。`t` キーはアプリ再起動を伴うので、
    /// これを持ち越さないと track 数を変えるたびに chord mode が解除されてしまう。
    #[serde(default)]
    pub grid_sequencer_chord_mode: bool,
    /// 人間が編集した Grid Sequencer の行と AUTO / HOLD 状態。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_sequencer: Option<crate::GridSequencerSessionState>,
    /// Grid Sequencer の手動BPM。`None` は既定の自動（BPM130）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_sequencer_bpm: Option<f64>,
    /// Loop Browser の手動BPM。`None` は配置clipからの自動選択。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_browser_bpm: Option<f64>,
    /// Grid Sequencer の自動BPMを引く範囲 `[最小, 最大]`。`None` は既定の固定値。
    /// 保存するのは範囲だけで、引いた BPM は起動のたびに引き直す。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_sequencer_bpm_range: Option<[f64; 2]>,
    /// Loop Browser の自動BPMを引く範囲 `[最小, 最大]`。`None` は既定の固定値。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_browser_bpm_range: Option<[f64; 2]>,
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
            active_screen: cmrt_tui_core::screen_switch::PrimaryScreen::Notepad,
            keyboard: KeyboardSessionState::default(),
            grid_sequencer_track_count: cmrt_realtime_play::DEFAULT_LIVE_INSTANCE_COUNT,
            grid_sequencer_chord_mode: false,
            grid_sequencer: None,
            grid_sequencer_bpm: None,
            loop_browser_bpm: None,
            grid_sequencer_bpm_range: None,
            loop_browser_bpm_range: None,
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
    active_screen: Option<cmrt_tui_core::screen_switch::PrimaryScreen>,
    #[serde(default)]
    is_daw_mode: bool,
    #[serde(default)]
    keyboard: Option<KeyboardSessionState>,
    #[serde(default = "default_grid_sequencer_track_count")]
    grid_sequencer_track_count: usize,
    #[serde(default)]
    grid_sequencer_chord_mode: bool,
    #[serde(default, deserialize_with = "deserialize_grid_sequencer")]
    grid_sequencer: Option<crate::GridSequencerSessionState>,
    #[serde(default)]
    grid_sequencer_bpm: Option<f64>,
    #[serde(default)]
    loop_browser_bpm: Option<f64>,
    #[serde(default)]
    grid_sequencer_bpm_range: Option<[f64; 2]>,
    #[serde(default)]
    loop_browser_bpm_range: Option<[f64; 2]>,
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
                cmrt_tui_core::screen_switch::PrimaryScreen::Keyboard
            } else if wire.is_daw_mode {
                cmrt_tui_core::screen_switch::PrimaryScreen::Daw
            } else {
                cmrt_tui_core::screen_switch::PrimaryScreen::Notepad
            }
        });
        Ok(Self {
            cursor: wire.cursor,
            lines: wire.lines,
            active_screen,
            keyboard: wire.keyboard.unwrap_or_default(),
            grid_sequencer_track_count: cmrt_realtime_play::normalize_live_instance_count(
                wire.grid_sequencer_track_count,
            ),
            grid_sequencer_chord_mode: wire.grid_sequencer_chord_mode,
            grid_sequencer: wire.grid_sequencer,
            grid_sequencer_bpm: valid_saved_bpm(wire.grid_sequencer_bpm),
            loop_browser_bpm: valid_saved_bpm(wire.loop_browser_bpm),
            grid_sequencer_bpm_range: valid_saved_bpm_range(wire.grid_sequencer_bpm_range),
            loop_browser_bpm_range: valid_saved_bpm_range(wire.loop_browser_bpm_range),
            keyboard_note_guide_overlay_date: wire.keyboard_note_guide_overlay_date,
            notepad_sound_check_guide_overlay_date: wire.notepad_sound_check_guide_overlay_date,
        })
    }
}

fn valid_saved_bpm(bpm: Option<f64>) -> Option<f64> {
    bpm.and_then(cmrt_tui_core::bpm::valid_bpm)
}

/// 保存済みの自動BPM範囲を、`BpmRange` として成立するものだけ通す。
fn valid_saved_bpm_range(range: Option<[f64; 2]>) -> Option<[f64; 2]> {
    let [minimum, maximum] = range?;
    cmrt_tui_core::bpm::BpmRange::new(minimum, maximum).map(|_| [minimum, maximum])
}

fn default_grid_sequencer_track_count() -> usize {
    cmrt_realtime_play::DEFAULT_LIVE_INSTANCE_COUNT
}

fn deserialize_grid_sequencer<'de, D>(
    deserializer: D,
) -> Result<Option<crate::GridSequencerSessionState>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
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
pub fn save_keyboard_note_guide_overlay_date(local_date: &str) -> Result<()> {
    let mut state = load_session_state();
    state.keyboard_note_guide_overlay_date = Some(local_date.to_owned());
    save_session_state(&state)
}

/// notepad の音出し確認 overlay の表示日だけを即時保存する。
pub fn save_notepad_sound_check_guide_overlay_date(local_date: &str) -> Result<()> {
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
