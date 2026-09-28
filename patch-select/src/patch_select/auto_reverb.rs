//! auto reverb の on/off（`e`）とルール overlay（`E`）の状態とキー操作。
//!
//! [`AutoReverbPanel`] は selector の実装に依らないので、自前の selector を持つ host も
//! 同じ操作と描画を使える。ルールを変えたら、保存と鳴らし直しを host へ頼む。
//! panel も selector も保存・送信はしない。

use super::*;

use cmrt_core::EffectPlugins;
use serde_json::Value;

use crate::auto_reverb::{resolve, reverb_candidates, AutoReverb, AutoReverbRules, HostChain};
use keys::{is_auto_reverb_rules_key, is_auto_reverb_toggle_key};

/// effect list の先頭に置く「掛けない」の表示。
const DRY_CHOICE_LABEL: &str = "なし(dry)";

/// [`PatchSelectRequest::auto_reverb`] に、auto reverb を扱う host が渡すもの。
#[derive(Clone, Default)]
pub struct AutoReverbHost {
    pub rules: AutoReverbRules,
    pub effect_plugins: EffectPlugins,
    /// 試聴する先（track・行）の effect chain。手動 reverb なら auto reverb は掛けない。
    pub chain: HostChain,
}

/// ルール overlay と on/off の状態。ルールは閉じるまで panel の中だけで変わる。
pub struct AutoReverbPanel {
    host: AutoReverbHost,
    /// ルール overlay に出す行を決める。
    user_presets: Vec<(String, String)>,
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
    /// effect list で試聴する effect が変わった（開いた・候補を動かした・閉じた）。host は
    /// 保存せず、カーソルの音色を [`AutoReverbPanel::audition_effect`] があればその effect で、
    /// 無ければ [`AutoReverbPanel::rules`] で鳴らし直す。
    Audition,
}

impl AutoReverbPanel {
    pub fn new(host: AutoReverbHost) -> Self {
        Self {
            host,
            user_presets: Vec::new(),
            overlay: None,
        }
    }

    /// selector の今のユーザー追加 preset。次に開くルール overlay から効く。
    pub fn set_user_presets(&mut self, user_presets: &[(String, String)]) {
        self.user_presets = user_presets.to_vec();
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

    fn has_manual_reverb(&self) -> bool {
        self.host.chain.is_manual_reverb()
    }

    pub(crate) fn overlay(&self) -> Option<&RulesOverlay> {
        self.overlay.as_ref()
    }

    /// effect list を開いている間、カーソルの音色の試聴に掛ける effect（`None` は dry）。
    /// 行・on/off・内蔵 effect に関わらず、選んでいる候補そのものを聴かせる。
    pub fn audition_effect(&self) -> Option<Option<&Value>> {
        let list = self.overlay.as_ref()?.effect_list.as_ref()?;
        Some(list.choices[list.cursor].effect.as_ref())
    }

    /// auto reverb のキーなら処理して結果を返す。overlay が開いている間は全キーを食う。
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<AutoReverbKey> {
        if self.overlay.is_some() {
            return Some(self.handle_overlay_key(key));
        }
        if is_auto_reverb_rules_key(key) {
            let shown_rows = self
                .host
                .rules
                .rows()
                .iter()
                .enumerate()
                .filter(|(_, (row, _))| row.is_shown(&self.user_presets))
                .map(|(index, _)| index)
                .collect();
            self.overlay = Some(RulesOverlay {
                shown_rows,
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
                    rules.set_effect(overlay.shown_rows[overlay.cursor], effect);
                    overlay.effect_list = None;
                }
                _ => {
                    let Some(delta) = vertical_delta(key) else {
                        return AutoReverbKey::Handled;
                    };
                    let cursor = move_cursor(list.cursor, list.choices.len(), delta);
                    if cursor == list.cursor {
                        return AutoReverbKey::Handled;
                    }
                    list.cursor = cursor;
                }
            }
            return AutoReverbKey::Audition;
        }
        match key.code {
            KeyCode::Esc => {
                let changed = overlay.opened_with != *rules;
                self.overlay = None;
                if changed {
                    return AutoReverbKey::RulesChanged;
                }
            }
            _ if is_open_effect_list_key(key) => {
                let current = rules.rows()[overlay.shown_rows[overlay.cursor]].1.as_ref();
                overlay.effect_list = Some(EffectList::open(catalog, current));
                return AutoReverbKey::Audition;
            }
            _ => {
                if let Some(delta) = vertical_delta(key) {
                    overlay.cursor = move_cursor(overlay.cursor, overlay.shown_rows.len(), delta);
                }
            }
        }
        AutoReverbKey::Handled
    }
}

/// ルール overlay。`opened_with` は閉じるときに変更の有無を比べるためのもの。
pub(crate) struct RulesOverlay {
    /// 出す行の、[`AutoReverbRules::rows`] での位置。`cursor` はこの並びの位置。
    shown_rows: Vec<usize>,
    cursor: usize,
    opened_with: AutoReverbRules,
    effect_list: Option<EffectList>,
}

impl RulesOverlay {
    pub(crate) fn shown_rows(&self) -> &[usize] {
        &self.shown_rows
    }

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
    /// 試聴先の chain が手動 reverb なので掛けない。`reverbs` は chain にある reverb の表示名。
    ManualReverb {
        reverbs: Vec<String>,
    },
    /// 絞り込みで音色が 1 つも無い。
    NoPatch,
}

fn move_cursor(cursor: usize, len: usize, delta: isize) -> usize {
    cursor
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}

/// ルールの行から effect list を開く。`x` のほか、決定・左右に見えるキーでも開く。
fn is_open_effect_list_key(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::NONE
        && matches!(
            key.code,
            KeyCode::Char('x' | ' ' | 'h' | 'l') | KeyCode::Enter | KeyCode::Left | KeyCode::Right
        )
}

fn vertical_delta(key: KeyEvent) -> Option<isize> {
    match (key.code, key.modifiers) {
        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => Some(-1),
        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => Some(1),
        (KeyCode::PageUp, _) => Some(-PAGE_STEP),
        (KeyCode::PageDown, _) => Some(PAGE_STEP),
        (KeyCode::Home, _) => Some(isize::MIN),
        (KeyCode::End, _) => Some(isize::MAX),
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
        self.filter_editing || self.auto_reverb_overlay_open() || self.plugin_menu.is_some()
    }

    /// `display` の試聴・確定で chain に入れる auto reverb の 1 段。掛けないとき
    /// （host 非対応・手動 reverb・dry・内蔵・off）は `None`。effect list を開いている間、
    /// カーソルの音色にはルールに代えて [`AutoReverbPanel::audition_effect`] を掛ける。
    pub fn auto_reverb_stage(&self, display: &str) -> Option<Value> {
        let panel = self.auto_reverb.as_ref()?;
        if panel.has_manual_reverb() {
            return None;
        }
        if let Some(effect) = panel.audition_effect() {
            if self.selected() == Some(display) {
                return effect.cloned();
            }
        }
        let entry = self.all.iter().find(|entry| entry.display() == display)?;
        match self.resolve_auto_reverb(panel, entry) {
            AutoReverb::Apply { stage, .. } => Some(stage),
            _ => None,
        }
    }

    pub(crate) fn auto_reverb_status(&self) -> Option<AutoReverbStatus> {
        let panel = self.auto_reverb.as_ref()?;
        if panel.has_manual_reverb() {
            return Some(AutoReverbStatus::ManualReverb {
                reverbs: panel
                    .host()
                    .chain
                    .reverb_labels(panel.host().effect_plugins.catalog()),
            });
        }
        if !panel.rules().enabled() {
            return Some(AutoReverbStatus::Resolved(AutoReverb::Off));
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
            // 同じ音色でも、掛ける effect が変わったので鳴らし直す。
            AutoReverbKey::Audition => Some(
                self.selected()
                    .map_or(PatchSelectAction::Continue, |patch| {
                        PatchSelectAction::Preview(patch.to_string())
                    }),
            ),
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
