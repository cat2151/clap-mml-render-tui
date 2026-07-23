use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::metadata::{validate_dir_id, validate_wav_id, LoopDirId, LoopWavId};

const RANDOM_DECK_VERSION: u32 = 1;
const LOOP_BROWSER_DIRECTORY: &str = "loop_browser";
const RANDOM_DECK_FILE_NAME: &str = "random_decks.toml";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LoopRandomScope {
    All,
    Favorites,
    FavoriteDir { dir: LoopDirId },
    Category { category: String },
}

impl LoopRandomScope {
    fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::All, Self::All) | (Self::Favorites, Self::Favorites) => true,
            (Self::FavoriteDir { dir: left }, Self::FavoriteDir { dir: right }) => {
                left.matches(right)
            }
            (Self::Category { category: left }, Self::Category { category: right }) => {
                left == right
            }
            _ => false,
        }
    }

    fn lookup_key(&self) -> LoopRandomScopeKey {
        match self {
            Self::All => LoopRandomScopeKey::All,
            Self::Favorites => LoopRandomScopeKey::Favorites,
            Self::FavoriteDir { dir } => {
                let (root, relative) = dir.lookup_key();
                LoopRandomScopeKey::FavoriteDir { root, relative }
            }
            Self::Category { category } => LoopRandomScopeKey::Category {
                category: category.clone(),
            },
        }
    }
}

#[derive(Eq, Hash, PartialEq)]
enum LoopRandomScopeKey {
    All,
    Favorites,
    FavoriteDir { root: String, relative: String },
    Category { category: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct StoredRandomDeck {
    scope: LoopRandomScope,
    order: Vec<LoopWavId>,
    next: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LoopRandomDeckState {
    last_selected: Option<LoopWavId>,
    decks: Vec<StoredRandomDeck>,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredRandomDeckState {
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_selected: Option<LoopWavId>,
    #[serde(default)]
    decks: Vec<StoredRandomDeck>,
}

impl LoopRandomDeckState {
    pub fn draw(
        &mut self,
        scope: LoopRandomScope,
        candidates: &[LoopWavId],
        current: Option<&LoopWavId>,
    ) -> Option<LoopWavId> {
        if candidates.is_empty() {
            return None;
        }
        let avoid = current.or(self.last_selected.as_ref()).cloned();
        let deck_index = self
            .decks
            .iter()
            .position(|deck| deck.scope.matches(&scope))
            .unwrap_or_else(|| {
                self.decks.push(new_deck(scope.clone(), candidates));
                self.decks.len() - 1
            });
        let deck = &mut self.decks[deck_index];
        if !same_candidate_set(&deck.order, candidates) {
            *deck = new_deck(scope, candidates);
        }
        if deck.next >= deck.order.len() {
            reshuffle(deck, candidates);
        }
        if candidates.len() > 1 {
            if let Some(avoid) = avoid.as_ref() {
                avoid_next(deck, candidates, avoid);
            }
        }
        let selected = deck.order.get(deck.next)?.clone();
        deck.next += 1;
        self.last_selected = Some(selected.clone());
        Some(selected)
    }
}

fn new_deck(scope: LoopRandomScope, candidates: &[LoopWavId]) -> StoredRandomDeck {
    let mut deck = StoredRandomDeck {
        scope,
        order: candidates.to_vec(),
        next: 0,
    };
    deck.order.shuffle(&mut rand::thread_rng());
    deck
}

fn reshuffle(deck: &mut StoredRandomDeck, candidates: &[LoopWavId]) {
    deck.order = candidates.to_vec();
    deck.order.shuffle(&mut rand::thread_rng());
    deck.next = 0;
}

fn avoid_next(deck: &mut StoredRandomDeck, candidates: &[LoopWavId], avoid: &LoopWavId) {
    if !deck
        .order
        .get(deck.next)
        .is_some_and(|candidate| candidate.matches(avoid))
    {
        return;
    }
    if let Some(replacement) =
        (deck.next + 1..deck.order.len()).find(|index| !deck.order[*index].matches(avoid))
    {
        deck.order.swap(deck.next, replacement);
        return;
    }
    reshuffle(deck, candidates);
    if deck.order[0].matches(avoid) {
        let replacement = deck
            .order
            .iter()
            .position(|candidate| !candidate.matches(avoid))
            .expect("multiple candidates must contain a different WAV");
        deck.order.swap(0, replacement);
    }
}

fn same_candidate_set(left: &[LoopWavId], right: &[LoopWavId]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let item_count = left.len();
    let left = left
        .iter()
        .map(LoopWavId::lookup_key)
        .collect::<HashSet<_>>();
    let right = right
        .iter()
        .map(LoopWavId::lookup_key)
        .collect::<HashSet<_>>();
    left.len() == item_count && right.len() == item_count && left == right
}

pub fn random_deck_path() -> Result<PathBuf> {
    crate::app_dir()
        .map(|dir| dir.join(LOOP_BROWSER_DIRECTORY).join(RANDOM_DECK_FILE_NAME))
        .ok_or_else(|| anyhow::anyhow!("システムの設定ディレクトリが取得できません"))
}

pub fn load_from(path: &Path) -> Result<LoopRandomDeckState> {
    if !path.exists() {
        return Ok(LoopRandomDeckState::default());
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("random deckを読めません: {}", path.display()))?;
    let stored: StoredRandomDeckState = toml::from_str(&text)
        .with_context(|| format!("random deckが壊れています: {}", path.display()))?;
    validate_stored(&stored)?;
    Ok(LoopRandomDeckState {
        last_selected: stored.last_selected,
        decks: stored.decks,
    })
}

pub fn save_to(path: &Path, state: &LoopRandomDeckState) -> Result<()> {
    let stored = StoredRandomDeckState {
        version: RANDOM_DECK_VERSION,
        last_selected: state.last_selected.clone(),
        decks: state.decks.clone(),
    };
    let text = toml::to_string_pretty(&stored)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("random deck directoryを作れません: {}", parent.display()))?;
    }
    atomic_write(path, text.as_bytes())
}

fn validate_stored(stored: &StoredRandomDeckState) -> Result<()> {
    if stored.version != RANDOM_DECK_VERSION {
        anyhow::bail!(
            "random deckのversionが一致しません（file: {}, expected: {}）",
            stored.version,
            RANDOM_DECK_VERSION
        );
    }
    if let Some(wav) = &stored.last_selected {
        validate_wav_id(wav)?;
    }
    let mut scope_keys = HashSet::with_capacity(stored.decks.len());
    for deck in &stored.decks {
        if !scope_keys.insert(deck.scope.lookup_key()) {
            anyhow::bail!("random deckに重複したscopeがあります");
        }
        if let LoopRandomScope::FavoriteDir { dir } = &deck.scope {
            validate_dir_id(dir)?;
        }
        if let LoopRandomScope::Category { category } = &deck.scope {
            if category.trim().is_empty() {
                anyhow::bail!("random deckに空のカテゴリscopeがあります");
            }
        }
        if deck.next > deck.order.len() {
            anyhow::bail!("random deckの次位置が範囲外です");
        }
        let mut wav_keys = HashSet::with_capacity(deck.order.len());
        for wav in &deck.order {
            validate_wav_id(wav)?;
            if !wav_keys.insert(wav.lookup_key()) {
                anyhow::bail!("random deckに重複したWAVがあります");
            }
        }
    }
    Ok(())
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let temp_path = unique_temp_path(path);
    let write_result = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .and_then(|mut file| {
            use std::io::Write as _;
            file.write_all(contents)?;
            file.sync_all()
        });
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("一時random deckを書けません: {}", temp_path.display()));
    }
    if let Err(error) = replace_file(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("random deckを置換できません: {}", path.display()));
    }
    Ok(())
}

fn unique_temp_path(path: &Path) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.with_extension(format!("tmp-{}-{nonce}", std::process::id()))
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let from = from
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let to = to
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}

#[cfg(test)]
mod tests;
