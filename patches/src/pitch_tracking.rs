//! 弾いた note に音高がついてくる（実測）drum percussion 行の音色を選ぶ照合。
//! 表は cat-music-patterns の `patch-pitch-tracking.json`。
//!
//! cmrt・cat-plugin-player・cat-smf-editor が同じ照合を使うよう、ここに 1 つだけ置く。
//! 表の取得（remote と cache）と、選んだ音色をどの欄へ出すかは各アプリが持つ。
//! 計測（server）には依存しない。

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Deserialize;

use crate::{DrumPatchRole, PatchRoleIndex, PatchRoleInput};

pub const PITCHED_PERCUSSION: &str = "pitched percussion";
pub const PITCH_TRACKING_FILE: &str = "patch-pitch-tracking.json";
pub const PITCH_TRACKING_URL: &str =
    "https://raw.githubusercontent.com/cat2151/cat-music-patterns/main/patch-pitch-tracking.json";
const FORMAT_VERSION: u32 = 1;

#[derive(Deserialize)]
struct File {
    format_version: u32,
    entries: Vec<Entry>,
}

/// 表のうち照合に使う列だけ。`observations` は読まない。
#[derive(Deserialize)]
struct Entry {
    plugin_id: String,
    display: String,
    tracks_keyboard: bool,
}

/// `tracks_keyboard == true` の音色を plugin_id ごとに持つ表。
#[derive(Debug, Default)]
pub struct PitchTrackingTable {
    tracking: HashMap<String, HashSet<String>>,
}

impl PitchTrackingTable {
    /// `format_version` が違う表は読まない（壊れた表で cache を上書きしないため）。
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let file: File = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if file.format_version != FORMAT_VERSION {
            return Err(format!(
                "unsupported format_version {}",
                file.format_version
            ));
        }
        let mut tracking: HashMap<String, HashSet<String>> = HashMap::new();
        for entry in file.entries.into_iter().filter(|e| e.tracks_keyboard) {
            tracking
                .entry(entry.plugin_id)
                .or_default()
                .insert(entry.display);
        }
        Ok(Self { tracking })
    }

    pub fn tracks_keyboard(&self, plugin_id: &str, display: &str) -> bool {
        self.tracking
            .get(plugin_id)
            .is_some_and(|displays| displays.contains(display))
    }

    /// (`plugin_id`, `display`) の一覧。順序は決まっていない。
    pub fn tracking(&self) -> impl Iterator<Item = (&str, &str)> {
        self.tracking.iter().flat_map(|(plugin_id, displays)| {
            displays
                .iter()
                .map(move |display| (plugin_id.as_str(), display.as_str()))
        })
    }
}

/// 照合する 1 音色。`group` が同じ音色どうしで分類索引を組む。
pub struct TrackingCandidate<'a, K> {
    pub group: K,
    pub plugin_id: Option<&'a str>,
    pub role: PatchRoleInput<'a>,
}

/// 表で true かつ drum の percussion 行に入る音色の index（昇順）。
///
/// [`PatchRoleIndex`] は display を key にするので、別 plugin に同じ display があると
/// 分類が後勝ちで混ざる。そのため `group`（plugin）ごとに組む。kick・snare・hi-hat 行は
/// 表で true でも選ばない。
pub fn pitched_percussion_indices<K: Ord>(
    candidates: &[TrackingCandidate<'_, K>],
    table: &PitchTrackingTable,
) -> Vec<usize> {
    let mut by_group: BTreeMap<&K, Vec<usize>> = BTreeMap::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if candidate
            .plugin_id
            .is_some_and(|plugin_id| table.tracks_keyboard(plugin_id, candidate.role.display))
        {
            by_group.entry(&candidate.group).or_default().push(index);
        }
    }
    let mut moved: Vec<usize> = by_group
        .values()
        .flat_map(|indices| {
            let roles =
                PatchRoleIndex::build(indices.iter().map(|&index| candidates[index].role), &[]);
            indices.iter().copied().filter(move |&index| {
                roles.drum_role_of(candidates[index].role.display)
                    == Some(DrumPatchRole::Percussion)
            })
        })
        .collect();
    moved.sort_unstable();
    moved
}

#[cfg(test)]
mod tests;
