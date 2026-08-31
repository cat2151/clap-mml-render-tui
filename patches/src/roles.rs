//! 最新の正規表現規則による、plugin非依存のpatch用途分類。
//!
//! serverが展開した`selector_category`と表示名を同じ条件へ通し、各patchを排他的な
//! [`PatchRole`]へ一度だけ割り当てる。MML selectorとGrid Sequencerは、この索引を共有する。

use std::collections::{HashMap, HashSet};

use regex::{Regex, RegexBuilder};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PatchRole {
    Bass,
    Chord,
    Lead,
    Drum,
    Triggered,
    Etc,
}

impl PatchRole {
    /// selectorで見せる順。分類時の優先順とは別。
    pub const ALL: [Self; 6] = [
        Self::Bass,
        Self::Chord,
        Self::Lead,
        Self::Drum,
        Self::Triggered,
        Self::Etc,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Bass => "bass",
            Self::Chord => "chord",
            Self::Lead => "lead",
            Self::Drum => "drum",
            Self::Triggered => "trigger",
            Self::Etc => "etc",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key {
            "bass" => Self::Bass,
            "chord" => Self::Chord,
            "lead" => Self::Lead,
            "drum" => Self::Drum,
            "trigger" => Self::Triggered,
            _ => Self::Etc,
        }
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|role| *role == self)
            .expect("PatchRole::ALL contains every role")
    }
}

/// GridのDrum各行が要求する、明示的な語。互いの残り集合ではない。
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum DrumPatchRole {
    Kick,
    Snare,
    HiHat,
    Percussion,
}

impl DrumPatchRole {
    pub const ALL: [Self; 4] = [Self::Kick, Self::Snare, Self::HiHat, Self::Percussion];

    pub fn pattern(self) -> &'static str {
        match self {
            Self::Kick => r"\bkick",
            Self::Snare => r"\bsnare",
            Self::HiHat => r"\bhat",
            Self::Percussion => r"\bperc",
        }
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|role| *role == self)
            .expect("DrumPatchRole::ALL contains every role")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PatchRolePreset {
    pub role: PatchRole,
    pub label: &'static str,
    pub pattern: &'static str,
}

/// 語頭境界はデータ側へ明示し、分類ロジックには音色名固有の補正を持ち込まない。
const BUILTIN_PRESETS: &[PatchRolePreset] = &[
    preset(PatchRole::Bass, "bass|bs", r"\b(?:bass|bs)"),
    preset(PatchRole::Chord, "strings", r"\bstrings?"),
    preset(PatchRole::Chord, "pad", r"\bpad"),
    preset(
        PatchRole::Chord,
        "keyboard|keys|piano",
        r"\b(?:keyboard|keys?|piano)",
    ),
    preset(PatchRole::Chord, "organ", r"\borgan"),
    preset(PatchRole::Chord, "guitar|gtr", r"\b(?:guitar|gtr)"),
    preset(PatchRole::Chord, "choir|vocal", r"\b(?:choir|vocal|voice)"),
    preset(PatchRole::Lead, "lead", r"\blead"),
    preset(PatchRole::Lead, "pluck", r"\bpluck"),
    preset(
        PatchRole::Lead,
        "woodwind",
        r"\b(?:flute|ocarina|wind|oboe|clarinet|bassoon|sax(?:ophone)?|piccolo|recorder|harmonica)",
    ),
    preset(
        PatchRole::Chord,
        "brass",
        r"\b(?:trumpet|brass|trombone|horn|tuba|flugelhorn|euphonium|cornet)",
    ),
    preset(PatchRole::Lead, "pizzicato", r"\bpizzicato"),
    preset(PatchRole::Lead, "bell", r"\bbell"),
    preset(PatchRole::Drum, "kick", r"\bkick"),
    preset(PatchRole::Drum, "snare", r"\bsnare"),
    preset(PatchRole::Drum, "hat", r"\bhat"),
    preset(PatchRole::Drum, "perc", r"\bperc"),
    preset(PatchRole::Drum, "drum", r"\bdrums?"),
    preset(PatchRole::Triggered, "chord", r"\bchord"),
    preset(
        PatchRole::Triggered,
        "arp|sequence",
        r"\b(?:arp|arpeggio|sequence|seq)",
    ),
    preset(PatchRole::Etc, "synth", r"\bsynth"),
    preset(
        PatchRole::Etc,
        "atmosphere",
        r"\b(?:atmosphere|ambiance|ambient|soundscape)",
    ),
    preset(PatchRole::Etc, "fx|effects", r"\b(?:fx\b|effects?|sfx)"),
];

const fn preset(role: PatchRole, label: &'static str, pattern: &'static str) -> PatchRolePreset {
    PatchRolePreset {
        role,
        label,
        pattern,
    }
}

pub fn builtin_role_presets() -> &'static [PatchRolePreset] {
    BUILTIN_PRESETS
}

/// 分類の排他的cascade。Etcはどこにも入らなかった残り。
///
/// Triggeredは表示名・categoryのどちらに現れても最優先。その後のRoleは、pluginが
/// 明示したselector categoryを表示名より先に評価する。
const CASCADE: [PatchRole; 5] = [
    PatchRole::Triggered,
    PatchRole::Drum,
    PatchRole::Bass,
    PatchRole::Chord,
    PatchRole::Lead,
];

#[derive(Clone, Copy)]
pub struct PatchRoleInput<'a> {
    pub display: &'a str,
    pub normalized_display: &'a str,
    pub selector_category: Option<&'a str>,
}

/// catalog順に依存しない、表示名からRoleと用途別候補を引く索引。
#[derive(Clone, Default)]
pub struct PatchRoleIndex {
    by_display: HashMap<String, PatchRole>,
    by_role: [Vec<String>; PatchRole::ALL.len()],
    by_drum_role: [Vec<String>; DrumPatchRole::ALL.len()],
}

impl PatchRoleIndex {
    pub fn build<'a>(
        entries: impl IntoIterator<Item = PatchRoleInput<'a>>,
        user_presets: &[(String, String)],
    ) -> Self {
        let user_presets = normalize_user_role_presets(user_presets.to_vec());
        let cascade = CASCADE.map(|role| compile_role(role, &user_presets));
        let drum_patterns = DrumPatchRole::ALL.map(|role| compile_condition(role.pattern()));
        let mut index = Self::default();
        for entry in entries {
            let role = classify_role(entry, &cascade);
            index.by_display.insert(entry.display.to_string(), role);
            index.by_role[role.index()].push(entry.display.to_string());
            if role == PatchRole::Drum {
                for (drum_role, condition) in DrumPatchRole::ALL.into_iter().zip(&drum_patterns) {
                    if condition_matches(condition, entry) {
                        index.by_drum_role[drum_role.index()].push(entry.display.to_string());
                    }
                }
            }
        }
        index
    }

    pub fn role_of(&self, display: &str) -> Option<PatchRole> {
        self.by_display.get(display).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.by_display.is_empty()
    }

    pub fn candidates(&self, role: PatchRole) -> &[String] {
        &self.by_role[role.index()]
    }

    pub fn drum_candidates(&self, role: DrumPatchRole) -> &[String] {
        &self.by_drum_role[role.index()]
    }
}

struct CompiledRole {
    role: PatchRole,
    alternatives: Vec<Vec<Regex>>,
}

impl CompiledRole {
    fn matches(&self, entry: PatchRoleInput<'_>) -> bool {
        self.alternatives
            .iter()
            .any(|condition| condition_matches(condition, entry))
    }

    fn matches_text(&self, text: &str) -> bool {
        self.alternatives
            .iter()
            .any(|condition| condition.iter().all(|regex| regex.is_match(text)))
    }
}

fn classify_role(entry: PatchRoleInput<'_>, cascade: &[CompiledRole]) -> PatchRole {
    let (triggered, roles) = cascade
        .split_first()
        .expect("classification cascade contains Triggered");
    debug_assert_eq!(triggered.role, PatchRole::Triggered);
    if triggered.matches(entry) {
        return PatchRole::Triggered;
    }

    if let Some(category) = entry.selector_category {
        if let Some(compiled) = roles
            .iter()
            .find(|compiled| compiled.matches_text(category))
        {
            return compiled.role;
        }
    }

    // 複数termのユーザー規則は、従来どおり表示名とcategoryをまたいでAND一致できる。
    roles
        .iter()
        .find(|compiled| compiled.matches(entry))
        .map_or(PatchRole::Etc, |compiled| compiled.role)
}

fn compile_role(role: PatchRole, user_presets: &[(String, String)]) -> CompiledRole {
    let alternatives = BUILTIN_PRESETS
        .iter()
        .filter(|preset| preset.role == role)
        .map(|preset| preset.pattern)
        .chain(
            user_presets
                .iter()
                .filter(move |(key, _)| PatchRole::from_key(key) == role)
                .map(|(_, pattern)| pattern.as_str()),
        )
        .map(compile_condition)
        .collect();
    CompiledRole { role, alternatives }
}

fn compile_condition(condition: &str) -> Vec<Regex> {
    condition
        .split_whitespace()
        .map(|term| {
            RegexBuilder::new(term)
                .case_insensitive(true)
                .build()
                .expect("validated role regular expression")
        })
        .collect()
}

fn condition_matches(condition: &[Regex], entry: PatchRoleInput<'_>) -> bool {
    condition.iter().all(|regex| {
        regex.is_match(entry.normalized_display)
            || entry
                .selector_category
                .is_some_and(|category| regex.is_match(category))
    })
}

pub fn normalize_user_role_presets(user_presets: Vec<(String, String)>) -> Vec<(String, String)> {
    let builtin_patterns = BUILTIN_PRESETS
        .iter()
        .map(|preset| preset.pattern)
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    user_presets
        .into_iter()
        .filter_map(|(group, pattern)| {
            let role = PatchRole::from_key(group.trim());
            let pattern = pattern.trim();
            let key = (role.key().to_string(), pattern.to_string());
            (!pattern.is_empty()
                && !builtin_patterns.contains(pattern)
                && is_valid_condition(pattern)
                && seen.insert(key.clone()))
            .then_some(key)
        })
        .collect()
}

fn is_valid_condition(condition: &str) -> bool {
    condition.split_whitespace().all(|term| {
        RegexBuilder::new(term)
            .case_insensitive(true)
            .build()
            .is_ok()
    })
}

#[cfg(test)]
mod tests;
