use std::cell::Cell;

use cmrt_core::AudioEffectCatalog;
use serde_json::Value;

mod add;

pub(crate) use add::{DawEffectAddState, EffectAddPane};

/// `PageDown`/`PageUp` の 1 回あたりの移動段数。chain 一覧・追加 overlay の両方で使う。
pub(crate) const PAGE_STEP: isize = 10;

/// EFFECT CHAIN overlay の編集中の状態。Enter まで init セルには書かない。
#[derive(Default)]
pub(crate) struct DawEffectChainOverlayState {
    pub(crate) track: usize,
    /// 1 行目に出す instrument の音色名（表示のみ）。
    pub(crate) instrument: String,
    /// chain の各段。配列の順 = 信号の順。要素は catalog が返した `{json_key: value}`。
    ///
    /// init セルに元からある要素は解釈せず値のまま持つ。catalog に無い要素も
    /// 表示（`effect_stage_label`）と削除はできるようにしておく。
    pub(crate) chain: Vec<Value>,
    pub(crate) cursor: usize,
    /// chain 一覧の表示先頭。描画側が上下 30% の余白の規則で更新する。
    pub(crate) scroll_offset: Cell<usize>,
    /// `dd` の 1 打目を受けた。
    pub(crate) pending_delete: bool,
    /// 追加 overlay（`a`）の category/kind/list 3 pane 状態。
    pub(crate) add: DawEffectAddState,
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
            chain,
            cursor: 0,
            scroll_offset: Cell::new(0),
            pending_delete: false,
            add: DawEffectAddState::default(),
            live_preview_command: None,
            preview_error: None,
        }
    }

    pub(crate) fn move_cursor(&mut self, delta: isize) {
        self.cursor = clamped_index(self.cursor, delta, self.chain.len());
    }

    pub(crate) fn push_stage(&mut self, stage: Value) {
        self.chain.push(stage);
        self.cursor = self.chain.len() - 1;
    }

    /// `index` 段を `stage` に差し替え、カーソルをその段へ置く。範囲外なら何もしない。
    pub(crate) fn replace_stage(&mut self, index: usize, stage: Value) {
        if let Some(slot) = self.chain.get_mut(index) {
            *slot = stage;
            self.cursor = index;
        }
    }

    pub(crate) fn delete_at_cursor(&mut self) -> bool {
        if self.cursor >= self.chain.len() {
            return false;
        }
        self.chain.remove(self.cursor);
        self.cursor = self.cursor.min(self.chain.len().saturating_sub(1));
        true
    }

    /// カーソル段の bypass を toggle する。chain が空なら何もせず `false` を返す。
    pub(crate) fn toggle_bypass_at_cursor(&mut self) -> bool {
        let Some(stage) = self.chain.get(self.cursor) else {
            return false;
        };
        let bypassed = crate::mml::effect_chain::stage_is_bypassed(stage);
        let next = crate::mml::effect_chain::stage_with_bypass(stage, !bypassed);
        self.chain[self.cursor] = next;
        true
    }

    /// カーソル段を `delta` 段だけ動かす（swap）。カーソルも付いていく。
    /// 端を越える移動は何もせず `false` を返す。
    pub(crate) fn move_stage(&mut self, delta: isize) -> bool {
        let len = self.chain.len();
        if len == 0 {
            return false;
        }
        let Some(target) = self.cursor.checked_add_signed(delta) else {
            return false;
        };
        if target >= len {
            return false;
        }
        self.chain.swap(self.cursor, target);
        self.cursor = target;
        true
    }
}

pub(crate) fn clamped_index(index: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    index.saturating_add_signed(delta).min(len - 1)
}

/// chain の 1 段を overlay に出す語。catalog にあれば `display`、無ければ JSON のまま
/// （何が書かれているか見えないと消す判断ができない）。bypass 段は前に `[bypass] ` を付ける。
pub(crate) fn effect_stage_label(stage: &Value, catalog: Option<&AudioEffectCatalog>) -> String {
    let known = stage.as_object().and_then(|object| {
        let mut plugin_keys = object
            .iter()
            .filter(|(key, _)| key.as_str() != cmrt_core::EFFECT_STAGE_BYPASS_JSON_KEY);
        let (json_key, value) = plugin_keys.next()?;
        if plugin_keys.next().is_some() {
            return None;
        }
        let value = value.as_str()?;
        let preset = catalog?.find(json_key, value).ok()?;
        Some(preset.display.clone())
    });
    let label = known.unwrap_or_else(|| stage.to_string());
    if crate::mml::effect_chain::stage_is_bypassed(stage) {
        format!("[bypass] {label}")
    } else {
        label
    }
}
