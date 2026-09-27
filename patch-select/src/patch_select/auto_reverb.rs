//! auto reverb のルール overlay（`e`）と on/off（`E`）の状態とキー操作。
//!
//! [`AutoReverbPanel`] は selector の実装に依らないので、自前の selector を持つ host も
//! 同じ操作と描画を使える。ルールを変えたら、保存と鳴らし直しを host へ頼む。
//! panel も selector も保存・送信はしない。

use super::*;

use cmrt_core::EffectPlugins;
use serde_json::Value;

use crate::auto_reverb::{resolve, reverb_candidates, AutoReverb, AutoReverbRules};
use keys::{is_auto_reverb_rules_key, is_auto_reverb_toggle_key};

/// effect list の先頭に置く「掛けない」の表示。
const DRY_CHOICE_LABEL: &str = "なし(dry)";

/// [`PatchSelectRequest::auto_reverb`] に、auto reverb を扱う host が渡すもの。
#[derive(Clone, Default)]
pub struct AutoReverbHost {
    pub rules: AutoReverbRules,
    pub effect_plugins: EffectPlugins,
    /// 試聴する先（track・行）に effect chain が既にある。あれば auto reverb は掛けない。
    pub existing_chain: bool,
}

/// ルール overlay と on/off の状態。ルールは閉じるまで panel の中だけで変わる。
pub struct AutoReverbPanel {
    host: AutoReverbHost,
    overlay: Option<RulesOverlay>,
}

/// [`AutoReverbPanel::handle_key`] が拾ったキーの結果。
#[derive(Debug, PartialEq, Eq)]
pub enum AutoReverbKey {
    /// 表示が変わっただけ。
    Handled,
    /// ルール（on/off を含む）が変わった。host は [`AutoReverbPanel::rules`] を保存し、
    /// カーソルの音色を今の設定で鳴らし直す。
    RulesChanged,
}

impl AutoReverbPanel {
    pub fn new(host: AutoReverbHost) -> Self {
        Self {
            host,
            overlay: None,
        }
    }

    pub fn rules(&self) -> &AutoReverbRules {
        &self.host.rules
    }

    /// ルール overlay が開いているか。開いている間、host は全キーを [`Self::handle_key`] へ渡す。
    pub fn overlay_open(&self) -> bool {
        self.overlay.is_some()
    }

    pub(crate) fn host(&self) -> &AutoReverbHost {
        &self.host
    }

    pub(crate) fn overlay(&self) -> Option<&RulesOverlay> {
        self.overlay.as_ref()
    }

    /// auto reverb のキーなら処理して結果を返す。overlay が開いている間は全キーを食う。
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<AutoReverbKey> {
        if self.overlay.is_some() {
            return Some(self.handle_overlay_key(key));
        }
        if is_auto_reverb_rules_key(key) {
            self.overlay = Some(RulesOverlay {
                cursor: 0,
                opened_with: self.host.rules.clone(),
                effect_list: None,
            });
            return Some(AutoReverbKey::Handled);
        }
        if is_auto_reverb_toggle_key(key) {
            let enabled = self.host.rules.enabled();
            self.host.rules.set_enabled(!enabled);
            return Some(AutoReverbKey::RulesChanged);
        }
        None
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) -> AutoReverbKey {
        let catalog = self.host.effect_plugins.catalog();
        let rules = &mut self.host.rules;
        let overlay = self
            .overlay
            .as_mut()
            .expect("called only while the rules overlay is open");
        if let Some(list) = overlay.effect_list.as_mut() {
            match key.code {
                KeyCode::Esc => overlay.effect_list = None,
                KeyCode::Enter => {
                    let effect = list.choices[list.cursor].effect.clone();
                    rules.set_effect(overlay.cursor, effect);
                    overlay.effect_list = None;
                }
                _ => {
                    if let Some(delta) = vertical_delta(key) {
                        list.cursor = move_cursor(list.cursor, list.choices.len(), delta);
                    }
                }
            }
            return AutoReverbKey::Handled;
        }
        match key.code {
            KeyCode::Esc => {
                let changed = overlay.opened_with != *rules;
                self.overlay = None;
                if changed {
                    return AutoReverbKey::RulesChanged;
                }
            }
            KeyCode::Char('x') if key.modifiers == KeyModifiers::NONE => {
                let current = rules.rows()[overlay.cursor].1.as_ref();
                overlay.effect_list = Some(EffectList::open(catalog, current));
            }
            _ => {
                if let Some(delta) = vertical_delta(key) {
                    overlay.cursor = move_cursor(overlay.cursor, rules.rows().len(), delta);
                }
            }
        }
        AutoReverbKey::Handled
    }
}

/// ルール overlay。`opened_with` は閉じるときに変更の有無を比べるためのもの。
pub(crate) struct RulesOverlay {
    cursor: usize,
    opened_with: AutoReverbRules,
    effect_list: Option<EffectList>,
}

impl RulesOverlay {
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn effect_list(&self) -> Option<&EffectList> {
        self.effect_list.as_ref()
    }
}

/// ルールの 1 行に紐付ける effect の候補。先頭が dry。
pub(crate) struct EffectList {
    choices: Vec<EffectChoice>,
    cursor: usize,
}

impl EffectList {
    fn open(catalog: Option<&cmrt_core::AudioEffectCatalog>, current: Option<&Value>) -> Self {
        let mut choices = vec![EffectChoice {
            label: DRY_CHOICE_LABEL.to_string(),
            effect: None,
        }];
        if let Some(catalog) = catalog {
            choices.extend(
                reverb_candidates(catalog)
                    .into_iter()
                    .map(|preset| EffectChoice {
                        label: preset.display.clone(),
                        effect: Some(preset.json_element()),
                    }),
            );
        }
        let cursor = choices
            .iter()
            .position(|choice| choice.effect.as_ref() == current)
            .unwrap_or(0);
        Self { choices, cursor }
    }

    pub(crate) fn choices(&self) -> &[EffectChoice] {
        &self.choices
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }
}

pub(crate) struct EffectChoice {
    pub(crate) label: String,
    pub(crate) effect: Option<Value>,
}

/// 表示行に出す、選択中の音色に対する auto reverb の状態。
#[derive(Debug, PartialEq)]
pub enum AutoReverbStatus {
    Resolved(AutoReverb),
    /// 試聴先の effect chain を優先して掛けない。
    ExistingChain,
    /// 絞り込みで音色が 1 つも無い。
    NoPatch,
}

fn move_cursor(cursor: usize, len: usize, delta: isize) -> usize {
    cursor
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}

fn vertical_delta(key: KeyEvent) -> Option<isize> {
    match (key.code, key.modifiers) {
        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => Some(-1),
        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => Some(1),
        _ => None,
    }
}

impl PatchSelect<'_> {
    /// auto reverb のルール overlay が開いているか。
    pub fn auto_reverb_overlay_open(&self) -> bool {
        self.auto_reverb
            .as_ref()
            .is_some_and(AutoReverbPanel::overlay_open)
    }

    /// host が selector の外のキー（演奏設定など）を拾わず、全キーを [`Self::handle_key`] へ渡すべきか。
    pub fn captures_all_keys(&self) -> bool {
        self.filter_editing || self.auto_reverb_overlay_open()
    }

    /// `display` の試聴・確定で chain に足す 1 段。掛けないとき（host 非対応・chain あり・dry・内蔵・off）は `None`。
    pub fn auto_reverb_stage(&self, display: &str) -> Option<Value> {
        let panel = self.auto_reverb.as_ref()?;
        if panel.host().existing_chain {
            return None;
        }
        let entry = self.all.iter().find(|entry| entry.display() == display)?;
        match self.resolve_auto_reverb(panel, entry) {
            AutoReverb::Apply { stage, .. } => Some(stage),
            _ => None,
        }
    }

    pub(crate) fn auto_reverb_status(&self) -> Option<AutoReverbStatus> {
        let panel = self.auto_reverb.as_ref()?;
        if !panel.rules().enabled() {
            return Some(AutoReverbStatus::Resolved(AutoReverb::Off));
        }
        if panel.host().existing_chain {
            return Some(AutoReverbStatus::ExistingChain);
        }
        let Some(index) = self.filtered.get(self.cursor) else {
            return Some(AutoReverbStatus::NoPatch);
        };
        Some(AutoReverbStatus::Resolved(
            self.resolve_auto_reverb(panel, &self.all[*index]),
        ))
    }

    pub(crate) fn auto_reverb_panel(&self) -> Option<&AutoReverbPanel> {
        self.auto_reverb.as_ref()
    }

    pub(crate) fn auto_reverb_rules(&self) -> Option<&AutoReverbRules> {
        self.auto_reverb.as_ref().map(AutoReverbPanel::rules)
    }

    #[cfg(test)]
    pub(crate) fn auto_reverb_overlay(&self) -> Option<&RulesOverlay> {
        self.auto_reverb.as_ref()?.overlay()
    }

    fn resolve_auto_reverb(
        &self,
        panel: &AutoReverbPanel,
        entry: &PatchCatalogEntry,
    ) -> AutoReverb {
        resolve(
            entry.display(),
            entry.has_builtin_effects(),
            &self.role_index,
            panel.host().effect_plugins.catalog(),
            panel.rules(),
        )
    }

    /// auto reverb のキーなら処理して action を返す。overlay が開いている間は全キーを食う。
    pub(super) fn handle_auto_reverb_key(&mut self, key: KeyEvent) -> Option<PatchSelectAction> {
        match self.auto_reverb.as_mut()?.handle_key(key)? {
            AutoReverbKey::Handled => Some(PatchSelectAction::Continue),
            AutoReverbKey::RulesChanged => Some(self.save_auto_reverb()),
        }
    }

    /// 今のルールの保存と、カーソルの音色の鳴らし直しを頼む。
    fn save_auto_reverb(&self) -> PatchSelectAction {
        PatchSelectAction::SaveAutoReverb {
            rules: self
                .auto_reverb_rules()
                .expect("saved only with auto reverb")
                .clone(),
            preview: self.selected().map(str::to_string),
        }
    }
}
