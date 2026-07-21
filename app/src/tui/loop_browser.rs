use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::loop_browser_metadata::{category_keys, LoopBrowserMetadata, LoopDirId, LoopWavId};
use crate::loop_browser_random::LoopRandomDeckState;
use crate::loop_browser_track_grid::{default_track_grid, LoopTrackClip, LoopTrackGrid};
use crate::loop_wav_analysis::LoopWavAnalysis;
use crate::loop_waveform::{LoopWaveform, WaveformDisplayScale};
use crate::tui::keyboard::NavigationCount;

mod catalog;
mod grid;
mod input;
mod mixer;
pub(super) mod performance;
pub(super) mod playback;
mod random_navigation;
mod reload;
mod screen;
mod track_input;
mod tree;

use tree::{collect_visible, find_favorite_node, insert_relative_path, node_path, sort_tree};

pub(in crate::tui) use screen::LoopBrowserScreen;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum LoopGridChange {
    #[default]
    Initial,
    Random,
    Pad(char),
    Category,
}

impl LoopGridChange {
    pub(super) fn label(self) -> String {
        match self {
            Self::Initial => "initial".to_string(),
            Self::Random => "random".to_string(),
            Self::Pad(pad) => format!("pad-{pad}"),
            Self::Category => "category".to_string(),
        }
    }
}

pub(crate) struct LoopBrowser {
    roots: Vec<(PathBuf, TreeNode)>,
    expanded: HashSet<NodeKey>,
    pub(super) visible: Vec<VisibleLoopNode>,
    pub(super) cursor: usize,
    pub(super) tree_scroll: usize,
    pub(super) error: Option<String>,
    pub(super) metadata_error: Option<String>,
    pub(super) track_grid_error: Option<String>,
    pub(super) random_decks_error: Option<String>,
    metadata: LoopBrowserMetadata,
    track_grid: LoopTrackGrid,
    track_volumes_db: Vec<i32>,
    solo_tracks: Vec<bool>,
    wav_analyses: Vec<(LoopWavId, LoopWavAnalysis)>,
    wav_waveforms: Vec<LoopWaveform>,
    waveform_display_scale: WaveformDisplayScale,
    wav_analysis_indices: HashMap<(String, String), usize>,
    wav_categories: HashMap<(String, String), String>,
    favorite_wav_keys: HashSet<(String, String)>,
    metadata_path: Option<PathBuf>,
    track_grid_path: Option<PathBuf>,
    random_decks: LoopRandomDeckState,
    random_decks_path: Option<PathBuf>,
    metadata_writable: bool,
    track_grid_writable: bool,
    pub(super) favorites_only: bool,
    pub(super) category_overlay: Option<LoopDirId>,
    pub(super) mixer_overlay_open: bool,
    pub(super) help_overlay: Option<LoopBrowserPane>,
    pub(super) mixer_cursor_track: usize,
    pub(super) category_keys: Vec<(char, String)>,
    pub(super) notice: Option<LoopBrowserNotice>,
    pub(super) focus: LoopBrowserPane,
    pub(super) track_cursor: usize,
    pub(super) measure_cursor: usize,
    pub(super) track_scroll: usize,
    pub(super) measure_scroll: usize,
    pub(super) used_wav_scroll: usize,
    pub(super) starting: bool,
    pub(super) playback_paused: bool,
    pub(super) stretch_diagnostics: playback::diagnostics::SharedLoopStretchDiagnostics,
    pub(super) playback_position: playback::position::SharedPlaybackPosition,
    pub(super) navigation_count: NavigationCount,
    pending_preview_trace: Option<u64>,
    pub(super) pending_render_trace: Option<u64>,
    pub(super) last_render_metrics: Option<performance::RenderMetrics>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct LoopPlaybackClip {
    pub(super) path: PathBuf,
    pub(super) span_measures: usize,
    pub(super) kind: crate::loop_wav_analysis::LoopWavKind,
    pub(super) bpm: Option<f64>,
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
        reason: LoopGridChange,
    },
    GridRefresh {
        grid: LoopPlaybackGrid,
        reason: LoopGridChange,
    },
    TrackVolumeChanged {
        track: usize,
        volume_db: i32,
    },
    TrackSoloChanged {
        solo_tracks: Vec<bool>,
    },
    SetPlaybackPaused {
        paused: bool,
        start_measure: usize,
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
            tree_scroll: 0,
            error: None,
            metadata_error: None,
            track_grid_error: None,
            random_decks_error: None,
            metadata: LoopBrowserMetadata::default(),
            track_grid: default_track_grid(),
            track_volumes_db: vec![0],
            solo_tracks: vec![false],
            wav_analyses: Vec::new(),
            wav_waveforms: Vec::new(),
            waveform_display_scale: WaveformDisplayScale::default(),
            wav_analysis_indices: HashMap::new(),
            wav_categories: HashMap::new(),
            favorite_wav_keys: HashSet::new(),
            metadata_path: None,
            track_grid_path: None,
            random_decks: LoopRandomDeckState::default(),
            random_decks_path: None,
            metadata_writable: true,
            track_grid_writable: true,
            favorites_only: false,
            category_overlay: None,
            mixer_overlay_open: false,
            help_overlay: None,
            mixer_cursor_track: 0,
            category_keys: Vec::new(),
            notice: None,
            focus: LoopBrowserPane::Tree,
            track_cursor: 0,
            measure_cursor: 0,
            track_scroll: 0,
            measure_scroll: 0,
            used_wav_scroll: 0,
            starting: false,
            playback_paused: false,
            stretch_diagnostics: playback::diagnostics::new_shared(),
            playback_position: playback::position::new_shared(),
            navigation_count: NavigationCount::default(),
            pending_preview_trace: None,
            pending_render_trace: None,
            last_render_metrics: None,
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
        let mut wav_waveforms = Vec::new();
        let mut wav_analysis_indices = HashMap::new();
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
                let analysis_index = wav_analyses.len();
                wav_analysis_indices
                    .entry(wav.lookup_key())
                    .or_insert(analysis_index);
                wav_analyses.push((wav, indexed_wav.analysis));
                wav_waveforms.push(indexed_wav.waveform);
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
        let waveform_display_scale = WaveformDisplayScale::from_waveforms(&wav_waveforms);
        let mut browser = Self {
            roots,
            expanded,
            category_keys: category_keys(categories),
            metadata,
            metadata_path,
            metadata_writable,
            metadata_error,
            wav_analyses,
            wav_waveforms,
            waveform_display_scale,
            wav_analysis_indices,
            ..Self::default()
        };
        browser.rebuild_wav_categories();
        browser.rebuild_favorite_wav_keys();
        browser.rebuild_visible(None);
        browser
    }

    fn selected_play_action(&mut self) -> LoopBrowserAction {
        let trace_id = performance::next_trace_id();
        self.selected_play_action_with_trace(trace_id)
    }

    fn selected_play_action_with_trace(&mut self, trace_id: u64) -> LoopBrowserAction {
        let path = self
            .visible
            .get(self.cursor)
            .filter(|node| node.is_wav)
            .map(|node| node.path.clone());
        if let Some(path) = path {
            self.pending_preview_trace = Some(trace_id);
            LoopBrowserAction::Preview(path)
        } else {
            LoopBrowserAction::Continue
        }
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
        self.tree_scroll = self.tree_scroll.min(self.visible.len().saturating_sub(1));
    }

    fn rebuild_visible_for_path(&mut self, selected: Option<&Path>) {
        self.rebuild_visible(None);
        if let Some(path) = selected {
            if let Some(index) = self.visible.iter().position(|node| node.path == path) {
                self.cursor = index;
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

    pub(super) fn take_preview_trace(&mut self) -> Option<u64> {
        self.pending_preview_trace.take()
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

    pub(super) fn solo_tracks(&self) -> &[bool] {
        &self.solo_tracks
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
