use cmrt_core::EffectPlugins;
use cmrt_patch_select::plugin_menu::PluginMenu;

use super::periodic_timeline::PeriodicTimeline;
use super::{
    KeyboardEffectPane, KeyboardMidiSender, KeyboardMmlInput, KeyboardNoteGuide,
    KeyboardPatchFilterInput, KeyboardState,
};

/// Keyboard画面が所有する接続・入力・表示状態。
pub struct KeyboardScreen<'a> {
    pub midi_sender: Option<KeyboardMidiSender>,
    pub state: KeyboardState,
    pub mml_input: KeyboardMmlInput<'a>,
    pub note_guide: KeyboardNoteGuide,
    pub patch_filter: KeyboardPatchFilterInput<'a>,
    pub(crate) plugin_menu: Option<PluginMenu>,
    effect_plugins: EffectPlugins,
    pub(crate) effect: KeyboardEffectPane,
    /// `y` でコピーした共有コマンド。`Some` の間は中央に通知を出す。
    pub(crate) share_notice: Option<String>,
    /// 周期送信を予約している live timeline。
    pub(crate) periodic_timeline: PeriodicTimeline,
}

impl<'a> KeyboardScreen<'a> {
    pub fn new(
        midi_sender: Option<KeyboardMidiSender>,
        state: KeyboardState,
        mml_input: KeyboardMmlInput<'a>,
        note_guide: KeyboardNoteGuide,
    ) -> Self {
        Self {
            midi_sender,
            state,
            mml_input,
            note_guide,
            patch_filter: KeyboardPatchFilterInput::default(),
            plugin_menu: None,
            effect_plugins: EffectPlugins::none(),
            effect: KeyboardEffectPane::default(),
            share_notice: None,
            periodic_timeline: PeriodicTimeline::default(),
        }
    }

    /// `effect_plugins` の catalog から、effect chain の候補を出す。
    pub fn with_effect_plugins(mut self, effect_plugins: EffectPlugins) -> Self {
        self.effect_plugins = effect_plugins;
        self
    }

    /// 保存されていた effect chain で始める。最初の音色の準備に同梱される。
    pub fn with_effect_chain(mut self, chain: Vec<serde_json::Value>) -> Self {
        self.effect = KeyboardEffectPane::restored(chain);
        self
    }

    pub fn effect_plugins(&self) -> &EffectPlugins {
        &self.effect_plugins
    }
}
