//! presetの条件。空白区切りのtermをANDで当てる。
//!
//! termは正規表現か、plugin条件のどちらか。plugin条件は正規表現ではなく、plugin名
//! （小文字・空白抜き）との完全一致: `plugin:dexed`はそのpluginの音色だけ、
//! `-plugin:dexed`はそのpluginの音色を除く。solo同士はOR、muteは全部を除く。
//! 表示パスにはplugin名が入らない音色があり、正規表現では否定も書けないため。

use regex::{Regex, RegexBuilder};

use super::PatchRoleInput;

const SOLO_PREFIX: &str = "plugin:";
const MUTE_PREFIX: &str = "-plugin:";

/// 条件の1 termがplugin条件なら、その種類とplugin名。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginTerm<'a> {
    Solo(&'a str),
    Mute(&'a str),
}

impl<'a> PluginTerm<'a> {
    pub fn parse(term: &'a str) -> Option<Self> {
        if let Some(slug) = term.strip_prefix(MUTE_PREFIX) {
            return (!slug.is_empty()).then_some(Self::Mute(slug));
        }
        let slug = term.strip_prefix(SOLO_PREFIX)?;
        (!slug.is_empty()).then_some(Self::Solo(slug))
    }

    pub fn to_term(self) -> String {
        match self {
            Self::Solo(slug) => format!("{SOLO_PREFIX}{slug}"),
            Self::Mute(slug) => format!("{MUTE_PREFIX}{slug}"),
        }
    }
}

/// plugin条件に書くplugin名。`Surge XT`なら`surgext`。
pub fn plugin_slug(plugin_name: &str) -> String {
    plugin_name
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

pub(super) struct Condition {
    solos: Vec<String>,
    mutes: Vec<String>,
    regexes: Vec<Regex>,
}

impl Condition {
    /// 検証済みの条件をcompileする。
    pub(super) fn compile(condition: &str) -> Self {
        let mut compiled = Self {
            solos: Vec::new(),
            mutes: Vec::new(),
            regexes: Vec::new(),
        };
        for term in condition.split_whitespace() {
            match PluginTerm::parse(term) {
                Some(PluginTerm::Solo(slug)) => compiled.solos.push(plugin_slug(slug)),
                Some(PluginTerm::Mute(slug)) => compiled.mutes.push(plugin_slug(slug)),
                None => compiled.regexes.push(
                    regex_term(term)
                        .build()
                        .expect("validated role regular expression"),
                ),
            }
        }
        compiled
    }

    /// plugin名が分からない音色は、soloには当たらずmuteでは除かれない。
    fn allows_plugin(&self, plugin: Option<&str>) -> bool {
        let slug = plugin.map(plugin_slug);
        let is = |candidate: &String| slug.as_ref() == Some(candidate);
        (self.solos.is_empty() || self.solos.iter().any(is)) && !self.mutes.iter().any(is)
    }

    /// 正規表現のtermは`text`だけに当てる。
    pub(super) fn matches_text(&self, plugin: Option<&str>, text: &str) -> bool {
        self.allows_plugin(plugin) && self.regexes.iter().all(|regex| regex.is_match(text))
    }

    /// 正規表現のtermは、termごとに表示名かcategoryのどちらかに当たればよい。
    pub(super) fn matches(&self, entry: PatchRoleInput<'_>) -> bool {
        self.allows_plugin(entry.plugin)
            && self.regexes.iter().all(|regex| {
                regex.is_match(entry.normalized_display)
                    || entry
                        .selector_category
                        .is_some_and(|category| regex.is_match(category))
            })
    }
}

pub(super) fn is_valid_condition(condition: &str) -> bool {
    condition
        .split_whitespace()
        .filter(|term| PluginTerm::parse(term).is_none())
        .all(|term| regex_term(term).build().is_ok())
}

fn regex_term(term: &str) -> RegexBuilder {
    let mut builder = RegexBuilder::new(term);
    builder.case_insensitive(true);
    builder
}
