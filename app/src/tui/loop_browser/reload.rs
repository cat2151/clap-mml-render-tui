use super::*;
use crate::loop_browser_metadata::metadata_path;
use crate::loop_browser_random::{load_from as load_random_decks, random_deck_path};
use crate::loop_browser_track_grid::{
    load_from as load_track_grid, normalize_previous_markers, reflow_with_spans, track_grid_path,
    LoadedTrackGrid,
};

impl LoopBrowser {
    pub(super) fn reload(&mut self, cfg: &crate::config::Config) {
        let (metadata, metadata_path, metadata_writable, metadata_error) = match metadata_path() {
            Ok(path) => match LoopBrowserMetadata::load_from(&path) {
                Ok(metadata) => (metadata, Some(path), true, None),
                Err(error) => (
                    LoopBrowserMetadata::default(),
                    Some(path),
                    false,
                    Some(error.to_string()),
                ),
            },
            Err(error) => (
                LoopBrowserMetadata::default(),
                None,
                false,
                Some(error.to_string()),
            ),
        };
        let (loaded_grid, track_grid_path, track_grid_writable, mut track_grid_error) =
            load_track_grid_state();
        let (random_decks, random_decks_path, random_decks_error) = load_random_deck_state();
        let mut browser = match crate::loop_library::load_index(cfg) {
            Ok(index) => Self::from_index(
                index,
                &cfg.loop_categories,
                metadata,
                metadata_path,
                metadata_writable,
                metadata_error,
            ),
            Err(error) => Self {
                error: Some(format!("{error}\ncmrt scan-loops を実行してください")),
                category_keys: category_keys(&cfg.loop_categories),
                metadata,
                metadata_path,
                metadata_writable,
                metadata_error,
                ..Self::default()
            },
        };
        let (reflowed_grid, _) = reflow_with_spans(&loaded_grid.grid, |wav| {
            browser
                .analysis_for_wav(wav)
                .map(|analysis| analysis.measures)
        });
        let (track_grid, _) = normalize_previous_markers(&reflowed_grid);
        let grid_changed = track_grid != loaded_grid.grid;
        if track_grid_writable && (loaded_grid.needs_migration || grid_changed) {
            if let Some(path) = track_grid_path.as_ref() {
                if let Err(error) = crate::loop_browser_track_grid::save_to(
                    path,
                    &track_grid,
                    &loaded_grid.track_volumes_db,
                ) {
                    track_grid_error =
                        Some(format!("track grid migrationを保存できません: {error}"));
                }
            }
        }
        browser.track_grid = track_grid;
        browser.track_volumes_db = loaded_grid.track_volumes_db;
        browser.solo_tracks.resize(browser.track_grid.len(), false);
        browser.track_grid_path = track_grid_path;
        browser.track_grid_writable = track_grid_writable;
        browser.track_grid_error = track_grid_error;
        browser.random_decks = random_decks;
        browser.random_decks_path = random_decks_path;
        browser.random_decks_error = random_decks_error;
        *self = browser;
    }
}

fn load_track_grid_state() -> (LoadedTrackGrid, Option<PathBuf>, bool, Option<String>) {
    match track_grid_path() {
        Ok(path) => match load_track_grid(&path) {
            Ok(loaded) => (loaded, Some(path), true, None),
            Err(error) => (
                empty_loaded_grid(),
                Some(path),
                false,
                Some(error.to_string()),
            ),
        },
        Err(error) => (empty_loaded_grid(), None, false, Some(error.to_string())),
    }
}

fn empty_loaded_grid() -> LoadedTrackGrid {
    LoadedTrackGrid {
        grid: default_track_grid(),
        track_volumes_db: vec![0],
        needs_migration: false,
    }
}

fn load_random_deck_state() -> (LoopRandomDeckState, Option<PathBuf>, Option<String>) {
    match random_deck_path() {
        Ok(path) => match load_random_decks(&path) {
            Ok(state) => (state, Some(path), None),
            Err(error) => (
                LoopRandomDeckState::default(),
                Some(path),
                Some(error.to_string()),
            ),
        },
        Err(error) => (
            LoopRandomDeckState::default(),
            None,
            Some(error.to_string()),
        ),
    }
}
