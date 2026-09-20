use cmrt_core::AudioEffectCatalog;
use serde_json::Value;

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
    /// `dd` の 1 打目を受けた。
    pub(crate) pending_delete: bool,
    /// 追加 overlay で選んでいる catalog の preset。
    pub(crate) add_cursor: usize,
}

impl DawEffectChainOverlayState {
    pub(crate) fn open(track: usize, instrument: String, chain: Vec<Value>) -> Self {
        Self {
            track,
            instrument,
            chain,
            cursor: 0,
            pending_delete: false,
            add_cursor: 0,
        }
    }

    pub(crate) fn move_cursor(&mut self, delta: isize) {
        self.cursor = clamped_index(self.cursor, delta, self.chain.len());
    }

    pub(crate) fn push_stage(&mut self, stage: Value) {
        self.chain.push(stage);
        self.cursor = self.chain.len() - 1;
    }

    pub(crate) fn delete_at_cursor(&mut self) -> bool {
        if self.cursor >= self.chain.len() {
            return false;
        }
        self.chain.remove(self.cursor);
        self.cursor = self.cursor.min(self.chain.len().saturating_sub(1));
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
/// （何が書かれているか見えないと消す判断ができない）。
pub(crate) fn effect_stage_label(stage: &Value, catalog: Option<&AudioEffectCatalog>) -> String {
    let known = stage.as_object().and_then(|object| {
        if object.len() != 1 {
            return None;
        }
        let (json_key, value) = object.iter().next()?;
        let value = value.as_str()?;
        let preset = catalog?.find(json_key, value).ok()?;
        Some(preset.display.clone())
    });
    known.unwrap_or_else(|| stage.to_string())
}
