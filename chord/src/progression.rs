//! コード進行カタログ（`cat-music-patterns` の `chord-progressions.json`）と、
//! degree 表記から note number 群への変換。

use anyhow::{Context, Result};
use chord2mml_core::{Event, ParsedItem};
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

/// chord2mml の構文・意味解析を通した、再利用可能なコード進行入力。
///
/// 音名・臨時記号・degree・quality はこの crate では解釈しない。Key の pitch class と
/// コード単位の元テキストは `chord2mml-core` の構造化 parse 結果をそのまま保持する。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedChordProgression {
    input: String,
    normalized_input: String,
    key_text: String,
    key_pitch_class: u8,
    chord_texts: Vec<String>,
    chords: Vec<Vec<u8>>,
}

impl ParsedChordProgression {
    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn normalized_input(&self) -> &str {
        &self.normalized_input
    }

    pub fn key_text(&self) -> &str {
        &self.key_text
    }

    pub fn key_pitch_class(&self) -> u8 {
        self.key_pitch_class
    }

    /// 画面やログで使う、12音へ正規化した Key 名。
    pub fn key_name(&self) -> &'static str {
        KEYS[usize::from(self.key_pitch_class)]
    }

    pub fn chord_texts(&self) -> &[String] {
        &self.chord_texts
    }

    pub fn chord_label(&self) -> String {
        self.chord_texts.join("-")
    }

    pub fn chords(&self) -> &[Vec<u8>] {
        &self.chords
    }
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
    let input = format!("Key:{key} {degrees}");
    parse_chord_progression(&input).map(|parsed| parsed.chords)
}

/// Key directiveを1つ持つコード進行を、構造化情報とnote number群へ変換する。
///
/// 文字列の構文解析とKeyのpitch class化は `chord2mml-core` が担当する。この関数が
/// 加えるのは、音楽アプリ向けの「Keyは先頭に1つ」「コード以外のeventは含めない」
/// 「1コード = 1つのnote群」という進行単位の制約だけ。
pub fn parse_chord_progression(input: &str) -> Result<ParsedChordProgression, String> {
    let input = input.trim();
    let parsed = chord2mml_core::parse(input)
        .map_err(|error| format!("コード進行を解釈できません（{input}）: {error}"))?;

    let mut key = None;
    let mut chord_texts = Vec::new();
    let mut first_chord_event = None;
    for item in parsed.items() {
        match item {
            ParsedItem::Key {
                text,
                pitch_class,
                event_index,
                ..
            } => {
                if key.is_some() {
                    return Err("Key指定は1つだけにしてください".to_string());
                }
                key = Some((text.clone(), *pitch_class, *event_index));
            }
            ParsedItem::Chord {
                text, event_index, ..
            } => {
                first_chord_event.get_or_insert(*event_index);
                chord_texts.push(text.clone());
            }
        }
    }
    let Some((key_text, key_pitch_class, key_event)) = key else {
        return Err("Key指定がありません（例: key:G Isus4-I）".to_string());
    };
    let Some(first_chord_event) = first_chord_event else {
        return Err("コード進行がありません（例: key:G Isus4-I）".to_string());
    };
    if key_event > first_chord_event {
        return Err("Key指定はコード進行より前に置いてください".to_string());
    }
    if parsed.events().iter().any(|event| {
        !matches!(
            event,
            Event::Key { .. }
                | Event::Chord(_)
                | Event::SlashChord(_)
                | Event::ChordOverBassNote(_)
                | Event::Inversion(_)
                | Event::Polychord(_)
        )
    }) {
        return Err("Key指定とコード進行以外のdirectiveは使用できません".to_string());
    }

    let mml = parsed
        .to_mml()
        .map_err(|error| format!("コード進行をMMLへ変換できません: {error}"))?;
    let chords = crate::mml_note_progression(&mml)?;
    if chords.len() != chord_texts.len() {
        return Err(format!(
            "コード数が一致しません（text={} notes={}）",
            chord_texts.len(),
            chords.len()
        ));
    }

    Ok(ParsedChordProgression {
        input: input.to_string(),
        normalized_input: parsed.normalized_input().to_string(),
        key_text,
        key_pitch_class,
        chord_texts,
        chords,
    })
}

#[cfg(test)]
mod tests;
