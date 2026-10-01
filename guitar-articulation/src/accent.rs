//! アクセントを付ける音の選び方。エコノミーピッキングと汚しの強弱で共用する。

use serde::{Deserialize, Serialize};

use crate::Note;

/// 折り返しのどこにアクセントを付けるか。フレーズの頭（先頭の列の音）はどれでもアクセント。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccentPattern {
    /// 上行から下行へ折り返す頂点。
    #[default]
    Top,
    /// 下行から上行へ折り返す谷。
    Bottom,
    /// 頂点と谷の両方。
    Both,
}

impl AccentPattern {
    /// 上だけ → 下だけ → 両方 → 上だけ。
    pub fn next(self) -> Self {
        match self {
            AccentPattern::Top => AccentPattern::Bottom,
            AccentPattern::Bottom => AccentPattern::Both,
            AccentPattern::Both => AccentPattern::Top,
        }
    }

    /// 画面に出す名前。
    pub fn label(self) -> &'static str {
        match self {
            AccentPattern::Top => "上",
            AccentPattern::Bottom => "下",
            AccentPattern::Both => "上下",
        }
    }

    pub fn is_default(&self) -> bool {
        *self == AccentPattern::default()
    }
}

/// アクセントの音。フレーズの頭（先頭の列の音）と、`pattern` の折り返しの音。
/// 折り返しは、前後の単音の列より高い（頂点）か低い（谷）単音。
pub(crate) fn accents(notes: &[Note], pattern: AccentPattern) -> Vec<bool> {
    let mut out: Vec<bool> = notes.iter().map(|note| note.column == 0).collect();
    let columns = crate::picking::columns(notes);
    for window in columns.windows(3) {
        if window.iter().any(|range| range.len() != 1) {
            continue;
        }
        let [before, turn, after] = [0, 1, 2].map(|k| notes[window[k].start].pitch);
        let top = turn > before && turn > after;
        let bottom = turn < before && turn < after;
        let hit = match pattern {
            AccentPattern::Top => top,
            AccentPattern::Bottom => bottom,
            AccentPattern::Both => top || bottom,
        };
        if hit {
            out[window[1].start] = true;
        }
    }
    out
}

#[cfg(test)]
mod tests;
