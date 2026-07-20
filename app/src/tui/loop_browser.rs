use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::loop_browser_metadata::{
    category_keys, metadata_path, LoopBrowserMetadata, LoopDirId, LoopWavId,
};
use crate::loop_browser_track_grid::{
    default_track_grid, load_from as load_track_grid, track_grid_path, LoopTrackGrid,
};
use crate::tui::keyboard::NavigationCount;

mod input;
pub(super) mod playback;
mod tree;

use tree::{collect_visible, find_favorite_node, insert_relative_path, node_path, sort_tree};

const REMOVED_NOTICE_DURATION: Duration = Duration::from_millis(1_500);
pub(super) const PAD_KEYS: [char; 7] = ['c', 'd', 'e', 'f', 'g', 'a', 'b'];

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NodeKey {
    root: usize,
    components: Vec<String>,
    anchor: Option<usize>,
}

#[derive(Clone, Debug)]
struct TreeNode {
    name: String,
    relative_path: PathBuf,
    children: Vec<TreeNode>,
    is_wav: bool,
}

#[derive(Clone, Debug)]
pub(super) struct VisibleLoopNode {
    key: NodeKey,
    pub(super) depth: usize,
    pub(super) name: String,
    pub(super) is_wav: bool,
    pub(super) expanded: bool,
    pub(super) path: PathBuf,
    pub(super) favorite: bool,
    pub(super) category: Option<String>,
}

pub(super) struct LoopBrowserNotice {
    pub(super) text: String,
    expires_at: Instant,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum LoopBrowserPane {
    #[default]
    Tree,
    Tracks,
}

pub(crate) struct LoopBrowser {
    roots: Vec<(PathBuf, TreeNode)>,
    expanded: HashSet<NodeKey>,
    pub(super) visible: Vec<VisibleLoopNode>,
    pub(super) cursor: usize,
    pub(super) list_state: ListState,
    pub(super) error: Option<String>,
    pub(super) metadata_error: Option<String>,
    pub(super) track_grid_error: Option<String>,
    metadata: LoopBrowserMetadata,
    track_grid: LoopTrackGrid,
    metadata_path: Option<PathBuf>,
    track_grid_path: Option<PathBuf>,
    metadata_writable: bool,
    track_grid_writable: bool,
    pub(super) favorites_only: bool,
    pub(super) category_overlay: Option<LoopDirId>,
    pub(super) category_keys: Vec<(char, String)>,
    pub(super) notice: Option<LoopBrowserNotice>,
    pub(super) focus: LoopBrowserPane,
    pub(super) track_cursor: usize,
    pub(super) measure_cursor: usize,
    pub(super) track_scroll: usize,
    pub(super) measure_scroll: usize,
    pub(super) navigation_count: NavigationCount,
}

pub(super) type LoopPlaybackGrid = Vec<Vec<Option<PathBuf>>>;

pub(super) enum LoopBrowserAction {
    Continue,
    Preview(PathBuf),
    Trigger {
        pad: char,
        path: PathBuf,
    },
    GridChanged {
        pad: char,
        audition: PathBuf,
        grid: LoopPlaybackGrid,
    },
    Return,
    Quit,
}

impl Default for LoopBrowser {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            expanded: HashSet::new(),
            visible: Vec::new(),
            cursor: 0,
            list_state: ListState::default(),
            error: None,
            metadata_error: None,
            track_grid_error: None,
            metadata: LoopBrowserMetadata::default(),
            track_grid: default_track_grid(),
            metadata_path: None,
            track_grid_path: None,
            metadata_writable: true,
            track_grid_writable: true,
            favorites_only: false,
            category_overlay: None,
            category_keys: Vec::new(),
            notice: None,
            focus: LoopBrowserPane::Tree,
            track_cursor: 0,
            measure_cursor: 0,
            track_scroll: 0,
            measure_scroll: 0,
            navigation_count: NavigationCount::default(),
        }
    }
}

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
        let (track_grid, track_grid_path, track_grid_writable, track_grid_error) =
            match track_grid_path() {
                Ok(path) => match load_track_grid(&path) {
                    Ok(track_grid) => (track_grid, Some(path), true, None),
                    Err(error) => (
                        default_track_grid(),
                        Some(path),
                        false,
                        Some(error.to_string()),
                    ),
                },
                Err(error) => (default_track_grid(), None, false, Some(error.to_string())),
            };
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
        browser.track_grid = track_grid;
        browser.track_grid_path = track_grid_path;
        browser.track_grid_writable = track_grid_writable;
        browser.track_grid_error = track_grid_error;
        *self = browser;
    }

    pub(in crate::tui) fn from_index(
        index: crate::loop_library::LoopIndex,
        categories: &[String],
        metadata: LoopBrowserMetadata,
        metadata_path: Option<PathBuf>,
        metadata_writable: bool,
        metadata_error: Option<String>,
    ) -> Self {
        let mut roots = Vec::with_capacity(index.roots.len());
        let mut expanded = HashSet::new();
        for (root_index, indexed_root) in index.roots.into_iter().enumerate() {
            let root_path = PathBuf::from(&indexed_root.path);
            let mut root = TreeNode {
                name: indexed_root.path,
                relative_path: PathBuf::new(),
                children: Vec::new(),
                is_wav: false,
            };
            for relative in indexed_root.wav_files {
                insert_relative_path(&mut root, Path::new(&relative));
            }
            sort_tree(&mut root);
            expanded.insert(NodeKey {
                root: root_index,
                components: Vec::new(),
                anchor: None,
            });
            roots.push((root_path, root));
        }
        let mut browser = Self {
            roots,
            expanded,
            category_keys: category_keys(categories),
            metadata,
            metadata_path,
            metadata_writable,
            metadata_error,
            ..Self::default()
        };
        browser.rebuild_visible(None);
        browser
    }

    fn selected_play_action(&self) -> LoopBrowserAction {
        self.visible
            .get(self.cursor)
            .filter(|node| node.is_wav)
            .map(|node| LoopBrowserAction::Preview(node.path.clone()))
            .unwrap_or(LoopBrowserAction::Continue)
    }

    fn rebuild_visible(&mut self, selected: Option<&NodeKey>) {
        let mut visible = Vec::new();
        if self.favorites_only {
            for (anchor, favorite) in self.metadata.favorite_dirs.iter().enumerate() {
                if let Some((root_index, root_path, node, components)) =
                    find_favorite_node(&self.roots, favorite)
                {
                    collect_visible(
                        root_index,
                        root_path,
                        node,
                        &self.expanded,
                        &self.metadata,
                        &self.category_keys,
                        components,
                        Some(anchor),
                        0,
                        Some(node_path(root_path, node).to_string_lossy().into_owned()),
                        &mut visible,
                    );
                }
            }
        } else {
            for (root_index, (root_path, root)) in self.roots.iter().enumerate() {
                collect_visible(
                    root_index,
                    root_path,
                    root,
                    &self.expanded,
                    &self.metadata,
                    &self.category_keys,
                    Vec::new(),
                    None,
                    0,
                    None,
                    &mut visible,
                );
            }
        }
        self.visible = visible;
        self.cursor = selected
            .and_then(|key| self.visible.iter().position(|node| &node.key == key))
            .unwrap_or_else(|| self.cursor.min(self.visible.len().saturating_sub(1)));
        self.list_state
            .select((!self.visible.is_empty()).then_some(self.cursor));
    }

    fn rebuild_visible_for_path(&mut self, selected: Option<&Path>) {
        self.rebuild_visible(None);
        if let Some(path) = selected {
            if let Some(index) = self.visible.iter().position(|node| node.path == path) {
                self.cursor = index;
                self.list_state.select(Some(index));
            }
        }
    }

    pub(super) fn active_notice(&mut self) -> Option<&LoopBrowserNotice> {
        if self
            .notice
            .as_ref()
            .is_some_and(|notice| Instant::now() >= notice.expires_at)
        {
            self.notice = None;
        }
        self.notice.as_ref()
    }

    pub(super) fn category_overlay_current(&self) -> Option<&str> {
        self.category_overlay
            .as_ref()
            .and_then(|dir| self.metadata.category_for(dir))
    }

    pub(super) fn pad_path(&self, pad: char) -> Option<PathBuf> {
        self.metadata.pad(pad).map(LoopWavId::path)
    }

    pub(super) fn pad_file_name(&self, pad: char) -> Option<String> {
        self.pad_path(pad).and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
    }

    pub(super) fn track_grid(&self) -> &[Vec<Option<LoopWavId>>] {
        &self.track_grid
    }

    pub(super) fn cell_label(&self, wav: &LoopWavId) -> String {
        let pad = self
            .metadata
            .pad_assignments
            .iter()
            .find(|assignment| assignment.wav.matches(wav))
            .map(|assignment| assignment.pad.to_ascii_uppercase());
        let file_name = wav
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| wav.relative.clone());
        match pad {
            Some(pad) => format!("{pad}:{file_name}"),
            None => file_name,
        }
    }

    pub(super) fn playback_grid(&self) -> LoopPlaybackGrid {
        self.track_grid
            .iter()
            .map(|track| {
                track
                    .iter()
                    .map(|cell| cell.as_ref().map(LoopWavId::path))
                    .collect()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
