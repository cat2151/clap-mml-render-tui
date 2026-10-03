//! MML 入力欄。画面に入ったときに開き、確定で raw・Articulated・matrix を作り直す。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{GuitarArticulationAction, GuitarArticulationScreen, Take, DEFAULT_MML};

impl GuitarArticulationScreen {
    /// 画面に入ったときに呼ぶ。MML 入力欄を開き、MML が空なら [`DEFAULT_MML`] を確定して鳴らす。
    /// サンプル MID を開いている間は何もしない（MID のキーを入力欄に取られないように）。
    /// SMF 素材の間も何もしない（MML 欄は素材を表していないので）。
    pub fn enter(&mut self) -> GuitarArticulationAction {
        if self.sample_midi.is_some() || self.smf.is_material() {
            return GuitarArticulationAction::Continue;
        }
        let action = if self.mml.is_empty() {
            match self.commit_mml(DEFAULT_MML) {
                Ok(()) => GuitarArticulationAction::Play(Take::Converted),
                Err(reason) => {
                    self.error = Some(reason);
                    GuitarArticulationAction::Continue
                }
            }
        } else {
            GuitarArticulationAction::Continue
        };
        self.open_input();
        action
    }

    pub(super) fn open_input(&mut self) {
        if self.input.is_none() {
            self.input = Some(cmrt_tui_core::text_input::new_single_line_textarea(
                &self.mml,
            ));
        }
    }

    pub(super) fn handle_input_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let Some(mut input) = self.input.take() else {
            return GuitarArticulationAction::Continue;
        };
        if key.code == KeyCode::Esc {
            self.error = None;
            return GuitarArticulationAction::Continue;
        }
        if is_commit_key(key) {
            let value = cmrt_tui_core::text_input::textarea_value(&input);
            return match self.commit_mml(value.trim()) {
                Ok(()) if !self.plain.is_empty() => GuitarArticulationAction::Play(Take::Converted),
                Ok(()) => GuitarArticulationAction::Continue,
                Err(reason) => {
                    // 閉じると打った文字列ごと消えるので、開いたまま理由を出す。
                    self.error = Some(reason);
                    self.input = Some(input);
                    GuitarArticulationAction::Continue
                }
            };
        }
        if cmrt_tui_core::text_input::apply_key_event_to_textarea(&mut input, key) {
            self.error = None;
        }
        self.input = Some(input);
        GuitarArticulationAction::Continue
    }

    /// MML を確定し、raw・Articulated・matrix を作り直して履歴へ積む。列ごとのルールは、付け替え元
    /// （[`crate::ColumnRuleAnchor`]）の列から新しい列へ付け替えたものにする（元が無ければ空）。付け替え元は変えない。
    /// 列に依らない行全体のルールは残す。空の MML は音を全部空にする。SMF 素材は捨てる。
    pub(super) fn commit_mml(&mut self, mml: &str) -> Result<(), String> {
        self.set_performance(mml, None)?;
        self.smf.discard_material();
        self.mml = mml.to_string();
        self.rules = match &self.anchor {
            Some(anchor) => anchor.rules_for(&self.notes, &self.rules),
            None => self.rules.without_column_rules(),
        };
        self.cursor = 0;
        self.error = None;
        self.rebuild_converted();
        self.record_history();
        Ok(())
    }
}

/// 1 行入力欄の確定キー。crossterm は `Ctrl+M` を `Enter` とは別に渡してくることがある。
pub(super) fn is_commit_key(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => true,
        KeyCode::Char('m') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}
