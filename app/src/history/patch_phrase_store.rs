use std::collections::{HashMap, HashSet};

use anyhow::Result;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PatchPhraseStore {
    #[serde(default)]
    pub notepad: PatchPhraseState,
    #[serde(default)]
    pub patches: HashMap<String, PatchPhraseState>,
    #[serde(default)]
    pub favorite_patches: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PatchPhraseState {
    #[serde(default)]
    pub history: Vec<String>,
    #[serde(default)]
    pub favorites: Vec<String>,
}

/// notepad / patch ごとの phrase history / favorites を patch_history.json に保存する。
pub fn save_patch_phrase_store(store: &PatchPhraseStore) -> Result<()> {
    let _ = super::paths::migrate_legacy_history_file("patch_history.json");
    let Some(path) = super::paths::patch_phrase_store_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(store)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// notepad / patch ごとの phrase history / favorites を patch_history.json から読み込む。
pub fn load_patch_phrase_store() -> PatchPhraseStore {
    let Some(path) = super::paths::resolved_history_file_path("patch_history.json") else {
        return PatchPhraseStore::default();
    };
    if !path.exists() {
        return PatchPhraseStore::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(crate) fn rename_patch_phrase_store_key(
    store: &mut PatchPhraseStore,
    from: &str,
    to: &str,
) -> bool {
    if from == to {
        return false;
    }
    let Some(source) = store.patches.remove(from) else {
        return false;
    };
    let has_favorites = {
        let dest = store.patches.entry(to.to_string()).or_default();
        super::helpers::merge_patch_phrase_items(&mut dest.history, source.history);
        super::helpers::merge_patch_phrase_items(&mut dest.favorites, source.favorites);
        !dest.favorites.is_empty()
    };

    if !has_favorites {
        store
            .favorite_patches
            .retain(|patch_name| patch_name != from && patch_name != to);
    } else if let Some(position) = store
        .favorite_patches
        .iter()
        .enumerate()
        .find_map(|(index, patch_name)| (patch_name == from || patch_name == to).then_some(index))
    {
        store
            .favorite_patches
            .retain(|patch_name| patch_name != from && patch_name != to);
        store.favorite_patches.insert(position, to.to_string());
    }

    true
}

pub(crate) fn normalize_patch_phrase_store_for_available_patches(
    store: &mut PatchPhraseStore,
    pairs: &[(String, String)],
) -> bool {
    let patch_names = store.patches.keys().cloned().collect::<Vec<_>>();
    let mut changed = false;
    for patch_name in patch_names {
        let Some(resolved) = crate::patches::resolve_display_patch_name(pairs, &patch_name) else {
            continue;
        };
        changed |= rename_patch_phrase_store_key(store, &patch_name, &resolved);
    }
    changed
}

pub(crate) fn touch_patch_favorite(store: &mut PatchPhraseStore, patch_name: &str) {
    if let Some(index) = store
        .favorite_patches
        .iter()
        .position(|existing| existing == patch_name)
    {
        if index == 0 {
            return;
        }
        store.favorite_patches.remove(index);
    }
    store.favorite_patches.insert(0, patch_name.to_string());
}

pub(crate) fn sync_patch_favorite_order(
    store: &mut PatchPhraseStore,
    patch_order: &[String],
) -> bool {
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();
    let mut changed = false;

    for patch_name in &store.favorite_patches {
        let is_favorite = store
            .patches
            .get(patch_name)
            .is_some_and(|state| !state.favorites.is_empty());
        if is_favorite && seen.insert(patch_name.clone()) {
            ordered.push(patch_name.clone());
            changed |= store.favorite_patches.get(ordered.len() - 1) != Some(patch_name);
        }
    }

    for patch_name in patch_order {
        let is_favorite = store
            .patches
            .get(patch_name)
            .is_some_and(|state| !state.favorites.is_empty());
        if is_favorite && seen.insert(patch_name.clone()) {
            ordered.push(patch_name.clone());
            changed |= store.favorite_patches.get(ordered.len() - 1) != Some(patch_name);
        }
    }

    let mut extras = store
        .patches
        .iter()
        .filter_map(|(patch_name, state)| {
            (!state.favorites.is_empty() && seen.insert(patch_name.clone()))
                .then_some(patch_name.clone())
        })
        .collect::<Vec<_>>();
    extras.sort_by(|left, right| crate::patches::compare_patch_names_natural(left, right));
    for patch_name in extras {
        changed |= store.favorite_patches.get(ordered.len()) != Some(&patch_name);
        ordered.push(patch_name);
    }
    changed |= store.favorite_patches.len() != ordered.len();

    if !changed {
        return false;
    }

    store.favorite_patches = ordered;
    true
}
