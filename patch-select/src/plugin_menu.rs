//! plugin を 1 キーで solo / mute する overlay（画面上の名前は plugin solo/mute）の中身。
//!
//! 絞り込み条件の文字列へ plugin 条件の term（`plugin:floe` / `-plugin:dexed`）を足し引きするだけで、
//! 絞り込みの規則は持たない（規則は [`crate::filter_candidates`]）。条件の文字列を持つ側が
//! この menu を持ち、[`PluginMenu::handle_key`] の結果で自分の条件を書き換える。
//!
//! 小文字キーは solo（その plugin だけ）、大文字キーは mute（その plugin を除く）。
//! solo も mute も重ねられるが、solo と mute は同時に置かない。
//! 置いてある term と同じものを選ぶと、その term を外す。

use std::collections::BTreeSet;

use cmrt_patches::{plugin_slug, PluginTerm};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::PatchCatalogEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginMode {
    Solo,
    Mute,
}

impl PluginMode {
    fn term(self, slug: &str) -> PluginTerm<'_> {
        match self {
            Self::Solo => PluginTerm::Solo(slug),
            Self::Mute => PluginTerm::Mute(slug),
        }
    }
}

pub struct PluginMenuItem {
    pub key: char,
    pub slug: String,
}

pub struct PluginMenu {
    items: Vec<PluginMenuItem>,
}

/// menu へ渡したキーの結果。
#[derive(Debug, PartialEq, Eq)]
pub enum PluginMenuKey {
    /// `Esc`。条件は変えずに閉じる。
    Close,
    /// menu に無いキー。開いたまま何もしない。
    Ignored,
    /// 選んだ後の条件。menu は閉じる。
    Chosen(String),
}

impl PluginMenu {
    /// catalog にある plugin を名前順に並べ、名前の中でまだ使われていない最初の英字をキーにする。
    /// 空くキーが無い plugin は menu に出さない。
    pub fn new(all: &[PatchCatalogEntry]) -> Self {
        let slugs: BTreeSet<String> = all
            .iter()
            .map(|patch| plugin_slug(patch.plugin_sort_key()))
            .filter(|slug| !slug.is_empty())
            .collect();
        let mut taken = BTreeSet::new();
        let items = slugs
            .into_iter()
            .filter_map(|slug| {
                let key = slug.chars().find(|character| {
                    character.is_ascii_lowercase() && !taken.contains(character)
                })?;
                taken.insert(key);
                Some(PluginMenuItem { key, slug })
            })
            .collect();
        Self { items }
    }

    pub fn items(&self) -> &[PluginMenuItem] {
        &self.items
    }

    /// `key` を、今の条件 `condition` に当てる。
    pub fn handle_key(&self, key: KeyEvent, condition: &str) -> PluginMenuKey {
        if key.code == KeyCode::Esc {
            return PluginMenuKey::Close;
        }
        let Some((choice, mode)) = menu_choice(key) else {
            return PluginMenuKey::Ignored;
        };
        match self.items.iter().find(|item| item.key == choice) {
            Some(item) => PluginMenuKey::Chosen(toggle_plugin_term(condition, &item.slug, mode)),
            None => PluginMenuKey::Ignored,
        }
    }
}

/// `condition` の plugin 条件を、`slug` を `mode` で選んだ後の形へ書き換える。plugin 以外の term はそのまま残す。
pub fn toggle_plugin_term(condition: &str, slug: &str, mode: PluginMode) -> String {
    let target = mode.term(slug);
    let terms: Vec<&str> = condition.split_whitespace().collect();
    let already = terms
        .iter()
        .any(|term| PluginTerm::parse(term) == Some(target));
    let mut kept: Vec<String> = terms
        .into_iter()
        .filter(|term| match PluginTerm::parse(term) {
            None => true,
            Some(plugin) if already => plugin != target,
            Some(PluginTerm::Mute(_)) => mode == PluginMode::Mute,
            Some(PluginTerm::Solo(_)) => mode == PluginMode::Solo,
        })
        .map(str::to_string)
        .collect();
    if !already {
        kept.push(target.to_term());
    }
    kept.join(" ")
}

/// `condition` で `slug` が solo / mute されているか。
pub fn plugin_mode(condition: &str, slug: &str) -> Option<PluginMode> {
    condition
        .split_whitespace()
        .find_map(|term| match PluginTerm::parse(term)? {
            PluginTerm::Solo(solo) if solo == slug => Some(PluginMode::Solo),
            PluginTerm::Mute(mute) if mute == slug => Some(PluginMode::Mute),
            _ => None,
        })
}

fn menu_choice(key: KeyEvent) -> Option<(char, PluginMode)> {
    let KeyCode::Char(character) = key.code else {
        return None;
    };
    if key.modifiers == KeyModifiers::NONE && character.is_ascii_lowercase() {
        return Some((character, PluginMode::Solo));
    }
    // 端末によって Shift が修飾に載るので、どちらも受ける。
    let shifted = key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT;
    (shifted && character.is_ascii_uppercase())
        .then(|| (character.to_ascii_lowercase(), PluginMode::Mute))
}
