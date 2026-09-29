use cmrt_effect_chain_select::EffectChainEditor;
use serde_json::Value;

/// EFFECT CHAIN overlay の編集中の状態。Enter まで init セルには書かない。
#[derive(Default)]
pub(crate) struct DawEffectChainOverlayState {
    pub(crate) track: usize,
    /// 1 行目に出す instrument の音色名（表示のみ）。
    pub(crate) instrument: String,
    /// init セルの chain の写し。元からある要素は解釈せず値のまま持つ。
    pub(crate) editor: EffectChainEditor,
    /// LIVE で鳴らした試聴の sender command。準備の成否が分かるまで持つ。
    pub(crate) live_preview_command: Option<u64>,
    /// 直近の試聴を鳴らせなかった理由。overlay に 1 行出し、次の試聴で消す。
    pub(crate) preview_error: Option<String>,
}

impl DawEffectChainOverlayState {
    pub(crate) fn open(track: usize, instrument: String, chain: Vec<Value>) -> Self {
        Self {
            track,
            instrument,
            editor: EffectChainEditor::open(chain),
            live_preview_command: None,
            preview_error: None,
        }
    }
}
