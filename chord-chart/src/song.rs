//! 曲（[`Song`]）と素材（[`Section`]）のデータモデル。

use serde::{Deserialize, Serialize};

/// prefix の既定値。chord2mml-rs がそのまま読める書式で、**この画面は解釈しない**。
pub(crate) const DEFAULT_PREFIX: &str = "Key=C BPM120";

/// section を指す不変の識別子。
///
/// arrangement が index ではなくこれを持つのは、section を削除したときに
/// arrangement 側の参照が全部ずれるのを避けるため。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SectionId(u32);

impl SectionId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// 曲の素材 1 つ（= 1 set のコード進行）。
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Section {
    pub id: SectionId,
    /// 画面に出す短い名前。`"A"` / `"B"` / `"Sabi"` / `"Intro"` など。
    pub name: String,
    /// degree 表記の進行。Key は曲に 1 つなので section は持たない。
    ///
    /// **解釈も検証もしない**（打った文字列をそのまま持つ）。書式は chord2mml-rs の
    /// ものをそのまま使うので、この画面でパースし直すのはムダ。読めない文字列を
    /// どうするかは演奏側の責任（別スコープ）。
    ///
    /// 中身の出どころはカタログ（`g` / `r`）か host 側の編集（`i`）だけ。
    /// **この crate は進行を 1 つも持たない**（既定の曲を作るためのハードコードもしない）。
    ///
    /// 何小節ぶんかは進行の記法（chord2mml の `|`）が持つ。この画面は別建ての
    /// 倍率を持たない。
    pub degrees: String,
}

impl Section {
    pub fn new(id: SectionId, name: impl Into<String>, degrees: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            degrees: degrees.into(),
        }
    }
}

/// 1 曲ぶんの構成。保存する曲は常に 1 つ（曲を選ぶブラウザはスコープ外）。
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Song {
    /// 曲の頭に置く chord2mml の指定（`"Key=C BPM120"`）。**解釈も検証もしない。**
    ///
    /// Key と BPM を別々のフィールドに割らないのは、書式が chord2mml-rs の
    /// 既存フォーマットそのものだから。分けるとムダなパースと組み立てが要る。
    pub prefix: String,
    pub sections: Vec<Section>,
    /// section の並び。同じ id が何度出てきてもよい。
    pub arrangement: Vec<SectionId>,
    /// 発番用のカウンタ。serde では持ち回らず、load 時に最大 id + 1 として復元する。
    #[serde(skip)]
    next_section_id: u32,
}

impl Default for Song {
    fn default() -> Self {
        Self::empty()
    }
}

impl Song {
    /// section を 1 つも持たない曲。
    ///
    /// **これが唯一の「初期値」**。曲の中身（コード進行）はカタログから引いたものしか
    /// 持たないので、既定の曲をここでハードコードすることはしない。保存ファイルが
    /// 無いときに 1 つ抽選するのは画面側（[`crate::ChordChartScreen::enter`]）の仕事。
    pub fn empty() -> Self {
        Self {
            prefix: DEFAULT_PREFIX.to_string(),
            sections: Vec::new(),
            arrangement: Vec::new(),
            next_section_id: 1,
        }
    }

    pub fn section(&self, id: SectionId) -> Option<&Section> {
        self.sections.iter().find(|section| section.id == id)
    }

    pub fn section_mut(&mut self, id: SectionId) -> Option<&mut Section> {
        self.sections.iter_mut().find(|section| section.id == id)
    }

    /// arrangement の並び順に、参照先の section を返す。
    pub fn arranged_sections(&self) -> impl Iterator<Item = &Section> {
        self.arrangement.iter().filter_map(|id| self.section(*id))
    }

    /// 新しい id を発番して section を末尾へ足す。
    pub fn push_section(
        &mut self,
        name: impl Into<String>,
        degrees: impl Into<String>,
    ) -> SectionId {
        let id = self.issue_section_id();
        self.sections.push(Section::new(id, name, degrees));
        id
    }

    /// 未使用の id を 1 つ発番する。
    pub fn issue_section_id(&mut self) -> SectionId {
        let id = SectionId::new(self.next_section_id.max(1));
        self.next_section_id = id.get().saturating_add(1);
        id
    }

    /// 発番カウンタを「今ある section の最大 id + 1」へ揃える。load 後に呼ぶ。
    pub fn reseed_section_ids(&mut self) {
        let max = self
            .sections
            .iter()
            .map(|section| section.id.get())
            .max()
            .unwrap_or(0);
        self.next_section_id = max.saturating_add(1).max(1);
    }
}

#[cfg(test)]
mod tests;
