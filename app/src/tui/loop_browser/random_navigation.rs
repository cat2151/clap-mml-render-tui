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
        let Some(selected) = self
            .random_decks
            .draw(scope.clone(), &candidates, current.as_ref())
        else {
            return LoopBrowserAction::Continue;
        };
        if let Err(error) = self.save_random_decks() {
            self.random_decks = previous;
            self.random_decks_error = Some(format!("random deckを保存できません: {error}"));
            return LoopBrowserAction::Continue;
        }
        self.random_decks_error = None;
        let analysis = self.analysis_for_wav(&selected);
        let category = self.category_for_wav(&selected).unwrap_or("none");
        let favorite = self
            .metadata
            .deepest_favorite_for_wav(&selected)
            .map_or("none", |(_, dir)| dir.relative.as_str());
        let (effective_bpm, declared_bpm, source) = analysis.map_or_else(
            || ("none".to_string(), "none".to_string(), "none"),
            |analysis| {
                let Some(tempo) = analysis.tempo else {
                    return ("none".to_string(), "none".to_string(), "one-shot");
                };
                (
                    crate::loop_time_stretch::format_bpm(tempo.bpm),
                    tempo
                        .declared_bpm
                        .map(crate::loop_time_stretch::format_bpm)
                        .unwrap_or_else(|| "none".to_string()),
                    match tempo.source {
                        crate::loop_wav_analysis::LoopAnalysisSource::Acid => "acid",
                        crate::loop_wav_analysis::LoopAnalysisSource::DurationEstimate => {
                            "duration-estimate"
                        }
                    },
                )
            },
        );
        super::playback::diagnostics::log_event(format!(
            "event=random-selected focus={:?} track={} measure={} scope={} candidates={} current=\"{}\" selected=\"{}\" favorite=\"{}\" category={} analysis_source={} declared_bpm={} effective_bpm={}",
            self.focus,
            self.track_cursor + 1,
            self.measure_cursor + 1,
            random_scope_label(&scope),
            candidates.len(),
            current
                .as_ref()
                .map(|wav| super::playback::diagnostics::path_label(&wav.path()))
                .unwrap_or_else(|| "none".to_string()),
            super::playback::diagnostics::path_label(&selected.path()),
            favorite.replace('"', "\\\""),
            category,
            source,
            declared_bpm,
            effective_bpm,
        ));
        let empty_track_cell = self.focus == LoopBrowserPane::Tracks && current.is_none();
        let updated_measure = match (self.focus, empty_track_cell) {
            (LoopBrowserPane::Tracks, true) => self.insert_current_clip(selected.clone()),
            (LoopBrowserPane::Tracks, false) => self.replace_current_clip(selected.clone()),
            (LoopBrowserPane::Tree, _) => None,
        };
        self.reveal_wav(&selected);
        let audition = selected.path();
        match updated_measure {
            Some(_) if empty_track_cell => LoopBrowserAction::GridRefresh {
                grid: self.playback_grid(),
                reason: LoopGridChange::Random,
            },
            Some(start_measure) => LoopBrowserAction::GridReplaced {
                start_measure,
                grid: self.playback_grid(),
                reason: LoopGridChange::Random,
            },
            None if empty_track_cell => LoopBrowserAction::Continue,
            None => {
                self.pending_preview_trace = Some(super::performance::next_trace_id());
                LoopBrowserAction::Preview(audition)
            }
        }
    }

    fn random_scope(&self) -> LoopRandomScope {
        match self.focus {
            LoopBrowserPane::Tree if self.favorites_only => LoopRandomScope::Favorites,
            LoopBrowserPane::Tree => LoopRandomScope::All,
            LoopBrowserPane::Tracks => self
                .clip_at(self.track_cursor, self.measure_cursor)
                .and_then(|(_, clip)| self.category_for_wav(&clip.wav))
                .map(|category| LoopRandomScope::Category {
                    category: category.to_string(),
                })
                .unwrap_or(LoopRandomScope::All),
        }
    }

    pub(in crate::tui) fn random_candidates(&self, scope: &LoopRandomScope) -> Vec<LoopWavId> {
        let mut candidates = Vec::new();
        let mut keys = HashSet::new();
        let favorite_dir_key = match scope {
            LoopRandomScope::FavoriteDir { dir } => Some(dir.lookup_key()),
            LoopRandomScope::All
            | LoopRandomScope::Favorites
            | LoopRandomScope::Category { .. } => None,
        };
        for (wav, _) in &self.wav_analyses {
            let key = wav.lookup_key();
            let included = match scope {
                LoopRandomScope::All => true,
                LoopRandomScope::Favorites => self.favorite_wav_keys.contains(&key),
                LoopRandomScope::FavoriteDir { .. } => favorite_dir_key
                    .as_ref()
                    .is_some_and(|dir| dir_key_contains_wav(dir, &key)),
                LoopRandomScope::Category { category } => self
                    .wav_categories
                    .get(&key)
                    .is_some_and(|candidate| candidate == category),
            };
            if included && keys.insert(key) {
                candidates.push(wav.clone());
            }
        }
        candidates
    }

    pub(super) fn rebuild_favorite_wav_keys(&mut self) {
        let favorite_dirs = self
            .metadata
            .favorite_dirs
            .iter()
            .map(LoopDirId::lookup_key)
            .collect::<Vec<_>>();
        self.favorite_wav_keys = self
            .wav_analyses
            .iter()
            .map(|(wav, _)| wav.lookup_key())
            .filter(|wav| {
                favorite_dirs
                    .iter()
                    .any(|favorite| dir_key_contains_wav(favorite, wav))
            })
            .collect();
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

    pub(super) fn reveal_wav(&mut self, wav: &LoopWavId) {
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

fn dir_key_contains_wav(dir: &(String, String), wav: &(String, String)) -> bool {
    if dir.0 != wav.0 {
        return false;
    }
    Path::new(&wav.1)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .starts_with(Path::new(&dir.1))
}

fn random_scope_label(scope: &LoopRandomScope) -> String {
    match scope {
        LoopRandomScope::All => "all".to_string(),
        LoopRandomScope::Favorites => "favorites".to_string(),
        LoopRandomScope::FavoriteDir { dir } => {
            format!("favorite-dir:{}", dir.relative.replace('"', "\\\""))
        }
        LoopRandomScope::Category { category } => {
            format!("category:{}", category.replace('"', "\\\""))
        }
    }
}
