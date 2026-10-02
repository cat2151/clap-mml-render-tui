//! 画面の「MML と設定のセット」の履歴と、専用 file（`guitar_articulation_history.json`）の読み書き。
//!
//! パスは `cmrt-history` が決め（`guitar_articulation_history_file_path`）、serde はこちらが持つ。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ArpSettings, ColumnRuleAnchor, RuleTable};

/// 履歴に残す件数の上限。notepad / DAW の履歴と揃える。
pub const HISTORY_MAX_LEN: usize = 100;

/// 履歴 1 件。音を決める状態と、MML を確定し直したときに列ルールを付け替える元を持つ（演奏結果は `mml`・`arp`・`rules` から作り直せる）。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GuitarArticulationHistoryEntry {
    #[serde(default)]
    pub mml: String,
    #[serde(default)]
    pub rules: RuleTable,
    #[serde(default)]
    pub effect_chain: Vec<Value>,
    /// `mml` に当てるアルペジエーターの設定。無い entry は OFF。
    #[serde(default)]
    pub arp: ArpSettings,
    /// MML を確定し直したときに列ルールを付け替える元。無い entry は `mml`・`arp` と `rules` の列ルールを元とみなす。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<ColumnRuleAnchor>,
}

/// 新しい順の履歴。同じ中身は重複させない。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GuitarArticulationHistory {
    #[serde(default)]
    pub entries: Vec<GuitarArticulationHistoryEntry>,
}

impl GuitarArticulationHistory {
    /// 先頭へ積む。既にある entry は先頭へ移し、上限を超えた最古を落とす。
    pub fn push_front(&mut self, entry: GuitarArticulationHistoryEntry) {
        self.entries.retain(|existing| existing != &entry);
        self.entries.insert(0, entry);
        self.entries.truncate(HISTORY_MAX_LEN);
    }
}

/// 保存済みの履歴を読む。file が無い・壊れている・保存先が決まらないときは空。
pub fn load_history() -> GuitarArticulationHistory {
    cmrt_history::guitar_articulation_history_file_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// 履歴を保存する。呼び出し側は失敗をログ 1 行に留め、画面は落とさないこと。
pub fn save_history(history: &GuitarArticulationHistory) -> anyhow::Result<()> {
    let path = cmrt_history::guitar_articulation_history_file_path().ok_or_else(|| {
        anyhow::anyhow!("guitar_articulation_history.json の保存先を決められない")
    })?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(history)?)?;
    Ok(())
}

#[cfg(test)]
mod tests;
