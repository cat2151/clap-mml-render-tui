use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::loop_browser_metadata::{category_keys, LoopBrowserMetadata, LoopDirId, LoopWavId};
use crate::loop_browser_random::LoopRandomDeckState;
use crate::loop_browser_track_grid::{default_track_grid, LoopTrackClip, LoopTrackGrid};
use crate::loop_wav_analysis::LoopWavAnalysis;
use crate::tui::keyboard::NavigationCount;

mod grid;
mod input;
pub(super) mod playback;
mod random_navigation;
mod reload;
mod track_input;
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
    analysis: Option<LoopWavAnalysis>,
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
    pub(super) analysis: Option<LoopWavAnalysis>,
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
    pub(super) random_decks_error: Option<String>,
    metadata: LoopBrowserMetadata,
    track_grid: LoopTrackGrid,
    track_volumes_db: Vec<i32>,
    wav_analyses: Vec<(LoopWavId, LoopWavAnalysis)>,
    metadata_path: Option<PathBuf>,
    track_grid_path: Option<PathBuf>,
    random_decks: LoopRandomDeckState,
    random_decks_path: Option<PathBuf>,
    metadata_writable: bool,
    track_grid_writable: bool,
    pub(super) favorites_only: bool,
    pub(super) category_overlay: Option<LoopDirId>,
    pub(super) mixer_overlay_open: bool,
    pub(super) mixer_cursor_track: usize,
    pub(super) category_keys: Vec<(char, String)>,
    pub(super) notice: Option<LoopBrowserNotice>,
    pub(super) focus: LoopBrowserPane,
    pub(super) track_cursor: usize,
    pub(super) measure_cursor: usize,
    pub(super) track_scroll: usize,
    pub(super) measure_scroll: usize,
    pub(super) navigation_count: NavigationCount,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct LoopPlaybackClip {
    pub(super) path: PathBuf,
    pub(super) span_measures: usize,
    pub(super) bpm: f64,
    pub(super) category: Option<String>,
    pub(super) meter_numerator: u16,
    pub(super) meter_denominator: u16,
}

pub(super) type LoopPlaybackGrid = Vec<Vec<Option<LoopPlaybackClip>>>;

pub(super) enum LoopBrowserAction {
    Continue,
    Preview(PathBuf),
    Trigger {
        pad: char,
        path: PathBuf,
    },
    GridReplaced {
        start_measure: usize,
        grid: LoopPlaybackGrid,
    },
    GridRefresh(LoopPlaybackGrid),
    TrackVolumeChanged {
        track: usize,
        volume_db: i32,
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
            random_decks_error: None,
            metadata: LoopBrowserMetadata::default(),
            track_grid: default_track_grid(),
            track_volumes_db: vec![0],
            wav_analyses: Vec::new(),
            metadata_path: None,
            track_grid_path: None,
            random_decks: LoopRandomDeckState::default(),
            random_decks_path: None,
            metadata_writable: true,
            track_grid_writable: true,
            favorites_only: false,
            category_overlay: None,
            mixer_overlay_open: false,
            mixer_cursor_track: 0,
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
    pub(in crate::tui) fn from_index(
        index: crate::loop_library::LoopIndex,
        categories: &[String],
        metadata: LoopBrowserMetadata,
        metadata_path: Option<PathBuf>,
        metadata_writable: bool,
        metadata_error: Option<String>,
    ) -> Self {
        let mut roots = Vec::with_capacity(index.roots.len());
        let mut wav_analyses = Vec::new();
        let mut expanded = HashSet::new();
        for (root_index, indexed_root) in index.roots.into_iter().enumerate() {
            let root_path = PathBuf::from(&indexed_root.path);
            let mut root = TreeNode {
                name: indexed_root.path,
                relative_path: PathBuf::new(),
                children: Vec::new(),
                is_wav: false,
                analysis: None,
            };
            for indexed_wav in indexed_root.wav_files {
                let wav = LoopWavId::new(&root_path, Path::new(&indexed_wav.relative));
                wav_analyses.push((wav, indexed_wav.analysis));
                insert_relative_path(
                    &mut root,
                    Path::new(&indexed_wav.relative),
                    indexed_wav.analysis,
                );
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
            wav_analyses,
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

    pub(super) fn track_grid(&self) -> &[Vec<Option<LoopTrackClip>>] {
        &self.track_grid
    }

    pub(super) fn track_volumes_db(&self) -> &[i32] {
        &self.track_volumes_db
    }

    pub(super) fn track_volume_db(&self, track: usize) -> i32 {
        self.track_volumes_db.get(track).copied().unwrap_or(0)
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
}

#[cfg(test)]
mod tests;
