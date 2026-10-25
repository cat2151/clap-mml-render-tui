//! 最新の正規表現規則による、plugin非依存のpatch用途分類。
//!
//! serverが展開した`selector_category`と表示名を同じ条件へ通し、各patchを排他的な
//! [`PatchRole`]へ一度だけ割り当てる。MML selectorとGrid Sequencerは、この索引を共有する。

use std::collections::{HashMap, HashSet};

use classify::{
    builtin_label_within_role, classify_role, levels, without_ignored_levels, CompiledRole,
};
use condition::{is_valid_condition, Condition};

use crate::pitch_tracking::PITCHED_PERCUSSION;
pub use condition::{plugin_slug, PluginTerm};

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

/// DrumはKick、Snare、HiHatの順に判定し、残りをPercussionへ分類する。
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
/// `bassdrum`（sfz側の綴り）・`Bass Drum`（Surge側の綴り）・`BASS DR. 2`（Dexed側の綴り）を
/// 1本で拾う。`dr`の後ろは`um`か語末に限り、`Bass Drive`を外す。
/// 略記の`bd`はカタログに1件も無いため、誤爆を避けて入れない。
///
/// **空白は`\s?`で書くこと。** [`compile_condition`]が条件を空白で分割してAND条件に
/// するので、リテラルの空白を混ぜるとgroupが割れて正規表現として壊れる。
const KICK_PATTERN: &str = r"\b(?:kick|bass\s?dr(?:um|\b))";
const SNARE_PATTERN: &str = r"\bsnare";
const HIHAT_PATTERN: &str = r"\b(?:hi[\s-]?)?hat";
/// 部位語の無い打楽器もPERC行の候補にする。`tom`と`ride`は語末に限り、作者名`Tomlyn`や`Rider`を外す。
/// `clap`は連結語の後半（`HandClap1`・`SPACECLAP`）に来るので、語中のどこでも当てる。
const PERCUSSION_PATTERN: &str =
    r"\b(?:perc|toms?\b|cymbal|crash|ride\b|\w*clap|conga|bongo|tabla|timbale|drums?)";

/// Bassの楽器語。全pluginで共通に使う。
macro_rules! bass_words {
    () => {
        // `synbass`の後ろの`o`は`SYNBASSOON`（木管）を外すため。
        r"\b(?:bass|bs|synbass(?:$|[^o])|synthbs)"
    };
}

/// pianoの楽器語。全pluginで共通に使う。
macro_rules! piano_words {
    () => {
        r"\b(?:keyboard|keys?|piano|rhodes|wurli|e?pno|elepiano|acpiano|grandpiano|harpsi|steinway|clavinet)"
    };
}

/// stringsの楽器語。全pluginで共通に使う。
macro_rules! strings_words {
    () => {
        r"\b(?:strings?|str(?:g|ng)|cello|violin|viola)"
    };
}

/// brassの楽器語。全pluginで共通に使う。
macro_rules! brass_words {
    () => {
        r"\b(?:trumpet|brass|trombone|horn|tuba|flugelhorn|euphonium|cornet|brs|synbrass|synbrs|tpt)"
    };
}

/// 語頭境界はデータ側へ明示し、分類ロジックには音色名固有の補正を持ち込まない。
///
/// Dexedの音色名は10文字に詰めた略記（`E.PNO`・`STRGS`・`SYN BRS`）や連結語（`SynBass`）が多いので、
/// その綴りも並べる。`regex`は先読みを持たないので、除外は直後の1文字の文字クラスで書く。
///
/// 並び順はselectorの欄の順で、同じRoleの中のlabelの優先順も兼ねる。1音色は先に当たった
/// 1つの欄にだけ入る（[`PatchRoleIndex::preset_label_of`]）。
const BUILTIN_PRESETS: &[PatchRolePreset] = &[
    preset(PatchRole::Bass, "bass|bs", bass_words!()),
    preset(PatchRole::Chord, "strings", strings_words!()),
    preset(PatchRole::Chord, "pad", r"\bpad"),
    preset(PatchRole::Chord, "keyboard|keys|piano", piano_words!()),
    preset(
        PatchRole::Chord,
        "organ",
        r"\b(?:organ|org\b|orgn|pipeorgn|eleorgan|hammond)",
    ),
    // guitarは連結語の後半（`EleGuitar1`・`NylonGtr.A`）に来るので、語中のどこでも当てる。
    preset(PatchRole::Chord, "guitar|gtr", r"\b\w*(?:guitar|gtr|guit)"),
    preset(
        PatchRole::Chord,
        "choir|vocal",
        r"\b(?:choir|vocal|voice|vox|hmnvoice|human)",
    ),
    preset(PatchRole::Lead, "lead", r"\blead"),
    // `vibra`の後ろの`t`は`vibrato`を外すため。
    // bellより前に置き、`Tubular Bell`・`Glock Bell`をmalletにする。
    // pluckより前に置き、弦を叩く`Hammered Dulcimer`をmalletにする。
    preset(
        PatchRole::Lead,
        "mallet",
        r"\b(?:mallet|marimba|xylo|vibe|vibra(?:$|[^t])|glock|celest|chime|music\s?box|tubular|dulcimer)",
    ),
    // `harp`の直後の除外は、harpsichordの略記（`HARPSICH`・`HARPSCD`・`HARPSY`・`HARPIC`・`Harpiano`）。
    preset(
        PatchRole::Lead,
        "pluck",
        r"\b(?:pluck|koto|harp(?:$|[^si]|s(?:$|[^ichy])|i(?:$|[^ca]))|sitar|shamisen|zither|kalimba|banjo|mandolin|lute)",
    ),
    preset(
        PatchRole::Lead,
        "woodwind",
        r"\b(?:flute|ocarina|wind|oboe|clarinet|bassoon|piccolo|recorder|harmonica|pan\s?flute|whistle|shakuhachi)",
    ),
    // saxはフルオーケストラの木管に入らないので、woodwindと別のlabelにしてreverbを分ける。
    preset(PatchRole::Lead, "sax", r"\bsax(?:ophone)?"),
    preset(PatchRole::Chord, "brass", brass_words!()),
    preset(PatchRole::Lead, "pizzicato", r"\bpizzicato"),
    preset(
        PatchRole::Lead,
        "pitched percussion",
        r"\bpitched\s?percussion\b",
    ),
    preset(PatchRole::Lead, "bell", r"\bbell"),
    preset(PatchRole::Drum, "kick", KICK_PATTERN),
    preset(PatchRole::Drum, "snare", SNARE_PATTERN),
    preset(PatchRole::Drum, "hi-hat", HIHAT_PATTERN),
    preset(PatchRole::Drum, "percussion", PERCUSSION_PATTERN),
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
/// Triggeredは表示名・categoryのどこに現れても最優先。残りは[`levels`]の階層ごとに通し、
/// 先の階層で当たったRoleが勝つ。同じ階層の中ではこの順で先勝ち。
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
    /// 音色を鳴らすplugin名。presetのplugin条件に使う。分からなければ`None`。
    pub plugin: Option<&'a str>,
}

/// catalog順に依存しない、表示名からRoleと用途別候補を引く索引。
#[derive(Clone, Default)]
pub struct PatchRoleIndex {
    by_display: HashMap<String, PatchRole>,
    drum_role_by_display: HashMap<String, DrumPatchRole>,
    preset_label_by_display: HashMap<String, &'static str>,
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
        let drum_patterns = DrumPatchRole::ALL.map(|role| Condition::compile(role.pattern()));
        let builtin_conditions: Vec<(&PatchRolePreset, Condition)> = BUILTIN_PRESETS
            .iter()
            .map(|preset| (preset, Condition::compile(preset.pattern)))
            .collect();
        let mut index = Self::default();
        for entry in entries {
            let levels = levels(entry);
            let normalized_display = without_ignored_levels(entry.normalized_display);
            let fallback = PatchRoleInput {
                normalized_display: &normalized_display,
                ..entry
            };
            let (mut role, decided_by) = classify_role(&levels, fallback, &cascade);
            // Explicit host-provided category: measured melodic percussion must
            // not lose to the general percussion word. Triggered stays first.
            let measured = role != PatchRole::Triggered
                && entry
                    .selector_category
                    .is_some_and(|category| category.eq_ignore_ascii_case(PITCHED_PERCUSSION));
            if measured {
                role = PatchRole::Lead;
            }
            index.by_display.insert(entry.display.to_string(), role);
            let label = if role == PatchRole::Drum {
                let drum_role = classify_drum_role(fallback, &drum_patterns);
                index
                    .drum_role_by_display
                    .insert(entry.display.to_string(), drum_role);
                index.by_drum_role[drum_role.index()].push(entry.display.to_string());
                Some(match drum_role {
                    DrumPatchRole::Kick => "kick",
                    DrumPatchRole::Snare => "snare",
                    DrumPatchRole::HiHat => "hi-hat",
                    DrumPatchRole::Percussion => "percussion",
                })
            } else if measured {
                Some(measured_label(fallback, &builtin_conditions))
            } else {
                builtin_label_within_role(decided_by, fallback, role, &builtin_conditions)
            };
            if let Some(label) = label {
                index
                    .preset_label_by_display
                    .insert(entry.display.to_string(), label);
            }
            index.by_role[role.index()].push(entry.display.to_string());
        }
        index
    }

    pub fn role_of(&self, display: &str) -> Option<PatchRole> {
        self.by_display.get(display).copied()
    }

    /// Drumの中の部位。部位語のないDrumはPercussion、Drum以外は`None`。
    pub fn drum_role_of(&self, display: &str) -> Option<DrumPatchRole> {
        self.drum_role_by_display.get(display).copied()
    }

    /// 決まったRole内のbuiltin presetのlabel。Drumは排他的な部位分類を使う。
    ///
    /// Roleはcascadeで先に決まり、labelはそのRoleの中だけで探す。Roleがユーザー定義presetだけで
    /// 決まったDrum以外の音色と、Etcのうちどのpresetにも当たらない音色では`None`。
    pub fn preset_label_of(&self, display: &str) -> Option<&'static str> {
        self.preset_label_by_display.get(display).copied()
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

/// 実測で移した音色の欄。名前で楽器が分かるmallet・bellは、その欄を優先する。
fn measured_label(
    fallback: PatchRoleInput<'_>,
    builtin_conditions: &[(&PatchRolePreset, Condition)],
) -> &'static str {
    let by_name = PatchRoleInput {
        selector_category: None,
        ..fallback
    };
    builtin_label_within_role(None, by_name, PatchRole::Lead, builtin_conditions)
        .filter(|label| matches!(*label, "mallet" | "bell"))
        .unwrap_or(PITCHED_PERCUSSION)
}

/// Drum内の部位を1つだけ決める。具体的な部位語を全体から探し、残りはPercussion。
/// categoryがPercussionでも、音色名のKick・Snare・Hatを優先する。
fn classify_drum_role(
    entry: PatchRoleInput<'_>,
    drum_patterns: &[Condition; DrumPatchRole::ALL.len()],
) -> DrumPatchRole {
    DrumPatchRole::ALL
        .into_iter()
        .zip(drum_patterns)
        .take(3)
        .find(|(_, condition)| condition.matches(entry))
        .map_or(DrumPatchRole::Percussion, |(drum_role, _)| drum_role)
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
        .map(Condition::compile)
        .collect();
    CompiledRole { role, alternatives }
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

mod classify;
mod condition;
#[cfg(test)]
mod tests;
