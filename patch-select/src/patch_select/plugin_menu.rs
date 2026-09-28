//! plugin を 1 キーで solo / mute する overlay（画面上の名前は plugin solo/mute）。
//!
//! 絞り込み欄へ plugin 条件の term（[`super::filter`]）を足し引きするだけで、絞り込みの規則は持たない。
//! 小文字キーは solo（その plugin だけ）、大文字キーは mute（その plugin を除く）。
//! solo も mute も重ねられるが、solo と mute は同時に置かない。
//! 置いてある term と同じものを選ぶと、その term を外す。

use std::collections::BTreeSet;

use cmrt_tui_core::text_input;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::filter::{plugin_slug, PluginTerm};
use super::{PatchSelect, PatchSelectAction};
use crate::PatchCatalogEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PluginMode {
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

pub(crate) struct PluginMenuItem {
    pub(crate) key: char,
    pub(crate) slug: String,
}

pub(crate) struct PluginMenu {
    items: Vec<PluginMenuItem>,
}

impl PluginMenu {
    /// catalog にある plugin を名前順に並べ、名前の中でまだ使われていない最初の英字をキーにする。
    /// 空くキーが無い plugin は menu に出さない。
    fn new(all: &[PatchCatalogEntry]) -> Self {
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

    pub(crate) fn items(&self) -> &[PluginMenuItem] {
        &self.items
    }
}

/// `query` の plugin 条件を、`slug` を `mode` で選んだ後の形へ書き換える。plugin 以外の term はそのまま残す。
fn toggle_plugin_term(query: &str, slug: &str, mode: PluginMode) -> String {
    let target = mode.term(slug);
    let terms: Vec<&str> = query.split_whitespace().collect();
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

impl PatchSelect<'_> {
    pub(crate) fn plugin_menu(&self) -> Option<&PluginMenu> {
        self.plugin_menu.as_ref()
    }

    /// 確定済みの絞り込みで `slug` が solo / mute されているか。
    pub(crate) fn plugin_mode(&self, slug: &str) -> Option<PluginMode> {
        self.committed_query
            .split_whitespace()
            .find_map(|term| match PluginTerm::parse(term)? {
                PluginTerm::Solo(solo) if solo == slug => Some(PluginMode::Solo),
                PluginTerm::Mute(mute) if mute == slug => Some(PluginMode::Mute),
                _ => None,
            })
    }

    pub(super) fn open_plugin_menu(&mut self) -> PatchSelectAction {
        self.plugin_menu = Some(PluginMenu::new(&self.all));
        PatchSelectAction::Continue
    }

    /// menu に無いキーは menu を開いたまま無視する。
    pub(super) fn handle_plugin_menu_key(&mut self, key: KeyEvent) -> PatchSelectAction {
        if key.code == KeyCode::Esc {
            self.plugin_menu = None;
            return PatchSelectAction::Continue;
        }
        let Some((choice, mode)) = menu_choice(key) else {
            return PatchSelectAction::Continue;
        };
        let Some(menu) = self.plugin_menu.take() else {
            return PatchSelectAction::Continue;
        };
        let Some(item) = menu.items.iter().find(|item| item.key == choice) else {
            self.plugin_menu = Some(menu);
            return PatchSelectAction::Continue;
        };
        self.committed_query = toggle_plugin_term(&self.committed_query, &item.slug, mode);
        self.query = text_input::new_single_line_textarea(&self.committed_query);
        self.refilter()
    }
}
