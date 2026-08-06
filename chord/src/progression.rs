//! コード進行カタログ（`cat-music-patterns` の `chord-progressions.json`）と、
//! degree 表記から note number 群への変換。

use anyhow::{Context, Result};
use rand::RngExt;
use serde::{Deserialize, Serialize};

/// Key 指定に使う12種のルート音名。chord2mml の `key` トークンは `[A-G][#＃♯]*[b♭]*`
/// を受けるので、♯表記だけで12種を網羅できる。
pub const KEYS: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// カタログの1件。JSON のオブジェクトと1対1で対応する。
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct ChordProgression {
    pub degrees: String,
    #[serde(default)]
    pub description: String,
}

impl ChordProgression {
    /// 進行に含まれるコードの数。`degrees` をハイフンで分割した個数。
    pub fn chord_count(&self) -> usize {
        self.degrees
            .split('-')
            .filter(|part| !part.trim().is_empty())
            .count()
    }
}

/// 抽選した進行を note number へ変換した結果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordProgressionPick {
    pub key: &'static str,
    pub degrees: String,
    /// コード1つぶんの note number 群を、進行の順に並べたもの。
    pub chords: Vec<Vec<u8>>,
}

/// コード進行カタログ。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChordProgressionCatalog {
    entries: Vec<ChordProgression>,
}

impl ChordProgressionCatalog {
    /// `chord-progressions.json`（トップレベルがオブジェクトの配列）を読む。
    pub fn from_json(text: &str) -> Result<Self> {
        let entries: Vec<ChordProgression> =
            serde_json::from_str(text).context("chord-progressions JSONの形式が不正です")?;
        if entries.is_empty() {
            anyhow::bail!("chord-progressions JSONにコード進行が1件もありません");
        }
        if let Some(invalid) = entries.iter().find(|entry| entry.chord_count() == 0) {
            anyhow::bail!("degrees が空のコード進行があります: {:?}", invalid);
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[ChordProgression] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 進行と Key をランダムに引き、note number へ変換できたものを返す。
    ///
    /// chord2mml が受け付けない degree 表記はカタログに混ざりうる（変換失敗時は
    /// 生 MML として素通しされるため、コード数の一致で検出する）。当たりが出る
    /// まで最大 `attempts` 回引き直し、全滅なら `None`。
    pub fn pick_playable(&self, attempts: usize) -> Option<ChordProgressionPick> {
        if self.entries.is_empty() {
            return None;
        }
        let mut rng = rand::rng();
        for _ in 0..attempts {
            let entry = &self.entries[rng.random_range(0..self.entries.len())];
            let key = KEYS[rng.random_range(0..KEYS.len())];
            if let Ok(chords) = chord_notes(&entry.degrees, key) {
                return Some(ChordProgressionPick {
                    key,
                    degrees: entry.degrees.clone(),
                    chords,
                });
            }
        }
        None
    }
}

/// ハイフン区切りの degree 表記を、指定 Key の note number 群へ変換する。
///
/// ハイフンは chord2mml が受け付ける区切り文字なので、そのまま渡す。
/// 解釈できない表記は `chord2mml_core::convert` が `Err` を返すので、
/// [`crate::note_progression`] の「失敗したら生 MML 扱い」フォールバックは通さない。
pub fn chord_notes(degrees: &str, key: &str) -> Result<Vec<Vec<u8>>, String> {
    let expected = degrees
        .split('-')
        .filter(|part| !part.trim().is_empty())
        .count();
    if expected == 0 {
        return Err("degrees が空です".to_string());
    }
    let input = format!("Key:{key} {degrees}");
    let mml = chord2mml_core::convert(&input)
        .map_err(|error| format!("コード進行を解釈できません（{input}）: {error}"))?;
    let chords = crate::mml_note_progression(&mml)?;
    if chords.len() != expected {
        // grid sequencer は「1コード = 1小節」で鳴らすので、degree の個数と和音の個数が
        // 1対1でないと小節の割り当てがずれる。変換自体の成否とは別の不変条件。
        return Err(format!(
            "コード数が一致しません（degrees={degrees} key={key} expected={expected} actual={}）",
            chords.len()
        ));
    }
    Ok(chords)
}

#[cfg(test)]
mod tests;
