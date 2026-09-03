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
///
/// Percussionは今も`\bperc`を明示的に要求する（残り物ではない）が、判定は排他的な
/// cascadeで、より具体的な部位が先に取る。表示パスにはフォルダ名も含まれるので、
/// `Percussion/Kick Clean.fxp`のように**部位語と`perc`が同時に当たる音色が実在する**。
/// 多重所属を許すと、そのkick・snare・hatがまとめてPERC行の候補にも化ける。
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum DrumPatchRole {
    Kick,
    Snare,
    HiHat,
    Percussion,
}

impl DrumPatchRole {
    /// 判定の優先順そのもの。具体的な部位から並べ、Percussionを最後に置く。
    pub const ALL: [Self; 4] = [Self::Kick, Self::Snare, Self::HiHat, Self::Percussion];

    /// ログや抽選デッキの識別子に使う短い綴り。[`PatchRole::key`]と衝突しない語にすること。
    pub fn key(self) -> &'static str {
        match self {
            Self::Kick => "kick",
            Self::Snare => "snare",
            Self::HiHat => "hat",
            Self::Percussion => "perc",
        }
    }

    pub fn pattern(self) -> &'static str {
        match self {
            Self::Kick => KICK_PATTERN,
            Self::Snare => SNARE_PATTERN,
            Self::HiHat => HIHAT_PATTERN,
            Self::Percussion => PERCUSSION_PATTERN,
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

/// バスドラムの綴り。`kick`と同義として扱う。
///
/// `bassdrum`（sfz側の綴り）と`Bass Drum`（Surge側の綴り）が実在するので、両方を1本で拾う。
/// 略記の`bd`はカタログに1件も無いため、誤爆を避けて入れない。
///
/// **空白は`\s?`で書くこと。** [`compile_condition`]が条件を空白で分割してAND条件に
/// するので、リテラルの空白を混ぜるとgroupが割れて正規表現として壊れる。
const KICK_PATTERN: &str = r"\b(?:kick|bass\s?drum)";
const SNARE_PATTERN: &str = r"\bsnare";
const HIHAT_PATTERN: &str = r"\bhat";
const PERCUSSION_PATTERN: &str = r"\bperc";

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
    preset(PatchRole::Drum, "kick|bass drum", KICK_PATTERN),
    preset(PatchRole::Drum, "snare", SNARE_PATTERN),
    preset(PatchRole::Drum, "hat", HIHAT_PATTERN),
    preset(PatchRole::Drum, "perc", PERCUSSION_PATTERN),
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
    drum_role_by_display: HashMap<String, DrumPatchRole>,
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
                if let Some(drum_role) = classify_drum_role(entry, &drum_patterns) {
                    index
                        .drum_role_by_display
                        .insert(entry.display.to_string(), drum_role);
                    index.by_drum_role[drum_role.index()].push(entry.display.to_string());
                }
            }
        }
        index
    }

    pub fn role_of(&self, display: &str) -> Option<PatchRole> {
        self.by_display.get(display).copied()
    }

    /// Drumの中の部位。Drum以外の音色と、部位語が当たらないDrumでは`None`。
    ///
    /// [`Self::role_of`]が`Drum`でも`None`になりうる（どの部位語にも当たらない
    /// `drums`だけの音色）ので、`Some(PatchRole::Drum)`から部位の存在を推測しないこと。
    pub fn drum_role_of(&self, display: &str) -> Option<DrumPatchRole> {
        self.drum_role_by_display.get(display).copied()
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

/// Drum内の部位を1つだけ決める。[`DrumPatchRole::ALL`]の順で先勝ち。
///
/// どれにも当たらなければ`None`（＝どの行の候補にもしない）。Percussionを残り物に
/// しないという方針は据え置きで、変えたのは「複数当たったときに誰が取るか」だけ。
fn classify_drum_role(
    entry: PatchRoleInput<'_>,
    drum_patterns: &[Vec<Regex>; DrumPatchRole::ALL.len()],
) -> Option<DrumPatchRole> {
    DrumPatchRole::ALL
        .into_iter()
        .zip(drum_patterns)
        .find(|(_, condition)| condition_matches(condition, entry))
        .map(|(drum_role, _)| drum_role)
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
