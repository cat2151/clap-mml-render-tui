use super::*;
use crate::loop_browser_random::{save_to as save_random_decks, LoopRandomScope};
use std::collections::HashSet;
use std::path::Component;

impl LoopBrowser {
    pub(super) fn select_random_wav(&mut self) -> LoopBrowserAction {
        let scope = self.random_scope();
        let candidates = self.random_candidates(&scope);
        if candidates.is_empty() {
            return LoopBrowserAction::Continue;
        }
        let current = self.selected_random_wav();
        let previous = self.random_decks.clone();
        let Some(selected) = self.random_decks.draw(scope, &candidates, current.as_ref()) else {
            return LoopBrowserAction::Continue;
        };
        if let Err(error) = self.save_random_decks() {
            self.random_decks = previous;
            self.random_decks_error = Some(format!("random deckを保存できません: {error}"));
            return LoopBrowserAction::Continue;
        }
        self.random_decks_error = None;
        let replaced_measure = (self.focus == LoopBrowserPane::Tracks && current.is_some())
            .then(|| self.replace_current_clip(selected.clone()))
            .flatten();
        self.reveal_wav(&selected);
        let audition = selected.path();
        if let Some(start_measure) = replaced_measure {
            LoopBrowserAction::GridReplaced {
                start_measure,
                grid: self.playback_grid(),
            }
        } else {
            LoopBrowserAction::Preview(audition)
        }
    }

    fn random_scope(&self) -> LoopRandomScope {
        match self.focus {
            LoopBrowserPane::Tree if self.favorites_only => LoopRandomScope::Favorites,
            LoopBrowserPane::Tree => LoopRandomScope::All,
            LoopBrowserPane::Tracks => self
                .clip_at(self.track_cursor, self.measure_cursor)
                .and_then(|(_, clip)| self.metadata.deepest_favorite_for_wav(&clip.wav))
                .map(|(_, dir)| LoopRandomScope::FavoriteDir { dir: dir.clone() })
                .unwrap_or(LoopRandomScope::All),
        }
    }

    pub(in crate::tui) fn random_candidates(&self, scope: &LoopRandomScope) -> Vec<LoopWavId> {
        let mut candidates = Vec::new();
        let mut keys = HashSet::new();
        for (wav, _) in &self.wav_analyses {
            let included = match scope {
                LoopRandomScope::All => true,
                LoopRandomScope::Favorites => self.metadata.deepest_favorite_for_wav(wav).is_some(),
                LoopRandomScope::FavoriteDir { dir } => dir.contains_wav(wav),
            };
            if included && keys.insert(wav.lookup_key()) {
                candidates.push(wav.clone());
            }
        }
        candidates
    }

    fn selected_tree_wav(&self) -> Option<LoopWavId> {
        let node = self.visible.get(self.cursor)?;
        if !node.is_wav {
            return None;
        }
        let root = &self.roots.get(node.key.root)?.0;
        let relative = node.key.components.iter().collect::<PathBuf>();
        Some(LoopWavId::new(root, &relative))
    }

    fn selected_random_wav(&self) -> Option<LoopWavId> {
        match self.focus {
            LoopBrowserPane::Tree => self.selected_tree_wav(),
            LoopBrowserPane::Tracks => self
                .clip_at(self.track_cursor, self.measure_cursor)
                .map(|(_, clip)| clip.wav.clone()),
        }
    }

    fn save_random_decks(&self) -> anyhow::Result<()> {
        let path = self
            .random_decks_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("random deckの保存先を取得できません"))?;
        save_random_decks(path, &self.random_decks)
    }

    fn reveal_wav(&mut self, wav: &LoopWavId) {
        let favorite = self
            .metadata
            .deepest_favorite_for_wav(wav)
            .map(|(index, dir)| (index, dir.depth()));
        if self.favorites_only && favorite.is_none() {
            self.favorites_only = false;
        }
        let anchor = self
            .favorites_only
            .then(|| favorite.map(|item| item.0))
            .flatten();
        let Some(root) = self
            .roots
            .iter()
            .position(|(root, _)| LoopWavId::new(root, Path::new(&wav.relative)).matches(wav))
        else {
            return;
        };
        let components = Path::new(&wav.relative)
            .components()
            .filter_map(|component| match component {
                Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let first_depth = anchor
            .and_then(|_| favorite.map(|item| item.1))
            .unwrap_or(0);
        for depth in first_depth..components.len() {
            self.expanded.insert(NodeKey {
                root,
                components: components[..depth].to_vec(),
                anchor,
            });
        }
        let selected = NodeKey {
            root,
            components,
            anchor,
        };
        self.rebuild_visible(Some(&selected));
    }
}
