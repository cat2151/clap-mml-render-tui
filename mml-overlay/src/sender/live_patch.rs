//! live instance に載せる音色と、その instance の出力に掛ける effect chain。
//!
//! server は音色と chain を同じ準備で差し替え、音色が今と同じで chain だけが違う準備では
//! 音色を読み直さない。つまり **chain 無しの準備を同じ音色で送ると、それだけで chain が外れる。**
//! 「その instance は準備済みか」は、音色と chain の両方が一致したときだけ真になる。

/// 音色と effect chain の組。`effect_chain` は MML 先頭 JSON の
/// `"effects after instrument"` の値の JSON 文字列で、空なら chain 無し。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LivePatch {
    patch: Option<String>,
    effect_chain: String,
}

impl LivePatch {
    /// chain 無しの音色。`None` なら server の既定音色。
    pub fn new(patch: Option<&str>) -> Self {
        Self {
            patch: patch.map(str::to_string),
            effect_chain: String::new(),
        }
    }

    /// 音色と effect chain。`effect_chain` が空なら [`Self::new`] と同じ。
    pub fn with_effect_chain(patch: Option<&str>, effect_chain: &str) -> Self {
        Self {
            patch: patch.map(str::to_string),
            effect_chain: effect_chain.to_string(),
        }
    }

    pub fn patch(&self) -> Option<&str> {
        self.patch.as_deref()
    }

    pub fn effect_chain(&self) -> &str {
        &self.effect_chain
    }

    /// log の 1 行へ埋める `patch=.. effect_chain=..`。
    pub(crate) fn log_fields(&self) -> String {
        format!(
            "patch={:?} effect_chain={:?}",
            self.patch, self.effect_chain
        )
    }
}

impl From<Option<&str>> for LivePatch {
    fn from(patch: Option<&str>) -> Self {
        Self::new(patch)
    }
}
