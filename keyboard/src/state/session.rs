use super::*;

impl KeyboardState {
    pub(crate) fn new(patch: Option<String>) -> Self {
        Self::from_session(KeyboardSessionState {
            patch,
            ..KeyboardSessionState::default()
        })
    }

    /// 音色を差し替えて作り直す。`t` のモードと対象、Patches pane の絞り込み条件は引き継ぐ。
    pub(crate) fn restart_with_patch(&self, patch: Option<String>) -> Self {
        Self::from_session(KeyboardSessionState {
            patch,
            ..self.session_state(String::new())
        })
    }

    /// 保存された状態から作る。`t` が off 以外なら、次の Ready で鳴り始める。
    pub fn from_session(session: KeyboardSessionState) -> Self {
        let mut state = Self::from_session_without_target(&session);
        state.note_playback_mode = session.note_playback_mode;
        let chords = if session.repeat_chords.iter().any(|chord| !chord.is_empty()) {
            session
                .repeat_chords
                .into_iter()
                .map(|chord| chord.into_iter().filter(|&note| note <= 127).collect())
                .collect()
        } else {
            default_repeat_chords(session.note_playback_mode)
        };
        let _ = state.replace_repeat_chords(chords, Instant::now(), false);
        state
    }

    fn from_session_without_target(session: &KeyboardSessionState) -> Self {
        Self {
            held: Vec::new(),
            patch: session
                .patch
                .clone()
                .and_then(|patch| (!patch.trim().is_empty()).then_some(patch)),
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
            sounding: SoundingTimeline::default(),
            periodic_next_at: None,
            periodic_anchor: None,
            periodic_generation: 0,
            repeat_elapsed_ticks: 0,
            combo_bag: None,
            current_combo: 0,
            refresh_pending: false,
            numeric_input: None,
            navigation_count: NavigationCount::default(),
            patch_catalog: KeyboardPatchCatalog::with_filter(&session.patch_filter),
        }
    }

    /// MML 入力欄と effect chain は `KeyboardScreen` が持つので、MML は呼び出し側から渡し、
    /// chain は呼び出し側が埋める。
    pub(crate) fn session_state(&self, mml: String) -> KeyboardSessionState {
        KeyboardSessionState {
            patch: self.patch.clone(),
            buffer_multiplier: self.buffer_multiplier,
            note_playback_mode: self.note_playback_mode,
            repeat_chords: self
                .repeat_chords
                .iter()
                .map(|chord| chord.iter().map(|note| note.midi_note).collect())
                .collect(),
            mml,
            effect_chain: Vec::new(),
            patch_filter: self.patch_catalog.filter().to_string(),
        }
    }
}

/// 保存された対象が無いときの既定。auto / arp はドファソ、repeat はド、off は無し。
pub(crate) fn default_repeat_chords(mode: NotePlaybackMode) -> Vec<Vec<u8>> {
    match mode {
        NotePlaybackMode::Auto | NotePlaybackMode::Arp => vec![vec![60, 65, 67]],
        NotePlaybackMode::Repeat => vec![vec![60]],
        NotePlaybackMode::Off => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
