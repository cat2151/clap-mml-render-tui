use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use cmrt_loop_browser_domain::metadata::{
    category_keys, LoopBrowserMetadata, LoopDirId, LoopWavId,
};
use cmrt_loop_browser_domain::persisted::PersistedDoc;
use cmrt_loop_browser_domain::random::LoopRandomDeckState;
use cmrt_loop_browser_domain::track_grid::{default_track_grid, LoopTrackClip, LoopTrackGrid};
use cmrt_loop_domain::loop_wav_analysis::LoopWavAnalysis;
use cmrt_loop_domain::loop_waveform::{LoopWaveform, WaveformDisplayScale};
use cmrt_tui_core::navigation::NavigationCount;

mod action;
mod batch_random;
mod catalog;
mod grid;
mod input;
mod mixer;
pub mod performance;
pub mod playback;
mod random_navigation;
mod reload;
mod screen;
mod track_input;
mod track_order;
mod tree;

pub use action::{LoopBrowserAction, LoopGridChange, LoopPlaybackClip, LoopPlaybackGrid};
use tree::{collect_visible, find_favorite_node, insert_relative_path, node_path, sort_tree};

pub mod ui;

#[cfg(any(test, feature = "test-support"))]
impl LoopBrowser {
    pub fn set_playback_beat_for_test(
        &self,
        measure: usize,
        beat: usize,
        beats_per_measure: usize,
    ) {
        playback::position::set_beat_for_test(
            &self.playback_position,
            measure,
            beat,
            beats_per_measure,
        );
    }
}

pub use playback::LoopPlaybackController;
pub use screen::LoopBrowserScreen;

use std::sync::OnceLock;

type LogSink = fn(&str);
static LOG_SINK: OnceLock<LogSink> = OnceLock::new();
static PERF_LOG_SINK: OnceLock<LogSink> = OnceLock::new();

/// app 起動時に、グローバルログ（`log/log.txt`）への書き込み関数を注入する。
/// `log` は同期書き込み、`perf_log` はレンダースレッドを塞がない非同期書き込みを想定する。
pub fn set_log_sinks(log: LogSink, perf_log: LogSink) {
    let _ = LOG_SINK.set(log);
    let _ = PERF_LOG_SINK.set(perf_log);
}

pub(crate) fn log_line(message: &str) {
    if let Some(sink) = LOG_SINK.get() {
        sink(message);
    }
}

pub(crate) fn perf_log_line(message: &str) {
    if let Some(sink) = PERF_LOG_SINK.get() {
        sink(message);
    }
}

/// loop browser 各ペインのキーバインド説明文（app のステータスバーからも参照）。
pub fn loop_browser_keybind_text(pane: LoopBrowserPane) -> &'static str {
    match pane {
        LoopBrowserPane::Tree => {
            "Ctrl+G:画面切替 Tab:track list p:演奏停止/再開 r:ランダムWAV Shift+C/D/E/F/G/A/B:pad登録/解除 c/d/e/f/g/a/b:pad演奏 1-9:hjkl prefix PgUp/PgDn:±10 t:dirカテゴリ v:dirお気に入り V:お気に入り限定 hjkl・矢印:移動/展開 Enter/Space:再生 q:終了"
        }
        LoopBrowserPane::Tracks => {
            "Ctrl+G:画面切替 Tab:loop tree p:演奏停止/再開 r:ランダムWAV m:mix level Shift+R:全track random Shift+M:2track random solo Alt+↓/↑:track並び替え 1-9:hjkl prefix s:solo toggle c..b:pad h/l・←/→:measure j/k・↓/↑:track（右/下端で追加） q:終了"
        }
    }
}

const REMOVED_NOTICE_DURATION: Duration = Duration::from_millis(1_500);
pub const PAD_KEYS: [char; 7] = ['c', 'd', 'e', 'f', 'g', 'a', 'b'];

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
pub struct VisibleLoopNode {
    key: NodeKey,
    pub depth: usize,
    pub name: String,
    pub is_wav: bool,
    pub expanded: bool,
    pub path: PathBuf,
    pub favorite: bool,
    pub category: Option<String>,
    pub analysis: Option<LoopWavAnalysis>,
}

pub struct LoopBrowserNotice {
    pub text: String,
    expires_at: Instant,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoopBrowserPane {
    #[default]
    Tree,
    Tracks,
}

pub struct LoopBrowser {
    roots: Vec<(PathBuf, TreeNode)>,
    expanded: HashSet<NodeKey>,
    pub visible: Vec<VisibleLoopNode>,
    pub cursor: usize,
    pub tree_scroll: usize,
    pub error: Option<String>,
    pub track_grid_error: Option<String>,
    metadata: PersistedDoc<LoopBrowserMetadata>,
    track_grid: LoopTrackGrid,
    track_volumes_db: Vec<i32>,
    solo_tracks: Vec<bool>,
    wav_analyses: Vec<(LoopWavId, LoopWavAnalysis)>,
    wav_waveforms: Vec<LoopWaveform>,
    waveform_display_scale: WaveformDisplayScale,
    wav_analysis_indices: HashMap<(String, String), usize>,
    wav_categories: HashMap<(String, String), String>,
    favorite_wav_keys: HashSet<(String, String)>,
    track_grid_path: Option<PathBuf>,
    random_decks: PersistedDoc<LoopRandomDeckState>,
    track_grid_writable: bool,
    pub favorites_only: bool,
    pub category_overlay: Option<LoopDirId>,
    pub mixer_overlay_open: bool,
    pub help_overlay: Option<LoopBrowserPane>,
    pub mixer_cursor_track: usize,
    pub category_keys: Vec<(char, String)>,
    pub notice: Option<LoopBrowserNotice>,
    pub focus: LoopBrowserPane,
    pub track_cursor: usize,
    pub measure_cursor: usize,
    pub track_scroll: usize,
    pub measure_scroll: usize,
    pub used_wav_scroll: usize,
    pub starting: bool,
    pub playback_paused: bool,
    pub stretch_diagnostics: playback::diagnostics::SharedLoopStretchDiagnostics,
    pub playback_position: playback::position::SharedPlaybackPosition,
    pub navigation_count: NavigationCount,
    pending_preview_trace: Option<u64>,
    pub pending_render_trace: Option<u64>,
    pub last_render_metrics: Option<performance::RenderMetrics>,
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
            track_grid_error: None,
            metadata: PersistedDoc::in_memory(LoopBrowserMetadata::default()),
            track_grid: default_track_grid(),
            track_volumes_db: vec![0],
            solo_tracks: vec![false],
            wav_analyses: Vec::new(),
            wav_waveforms: Vec::new(),
            waveform_display_scale: WaveformDisplayScale::default(),
            wav_analysis_indices: HashMap::new(),
            wav_categories: HashMap::new(),
            favorite_wav_keys: HashSet::new(),
            track_grid_path: None,
            random_decks: PersistedDoc::in_memory(LoopRandomDeckState::default()),
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
    pub fn from_index(
        index: cmrt_loop_browser_domain::library::LoopIndex,
        categories: &[String],
        metadata: PersistedDoc<LoopBrowserMetadata>,
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
            for (anchor, favorite) in self.metadata.value.favorite_dirs.iter().enumerate() {
                if let Some((root_index, root_path, node, components)) =
                    find_favorite_node(&self.roots, favorite)
                {
                    collect_visible(
                        root_index,
                        root_path,
                        node,
                        &self.expanded,
                        &self.metadata.value,
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
                    &self.metadata.value,
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

    pub fn active_notice(&mut self) -> Option<&LoopBrowserNotice> {
        if self
            .notice
            .as_ref()
            .is_some_and(|notice| Instant::now() >= notice.expires_at)
        {
            self.notice = None;
        }
        self.notice.as_ref()
    }

    pub fn take_preview_trace(&mut self) -> Option<u64> {
        self.pending_preview_trace.take()
    }

    pub fn category_overlay_current(&self) -> Option<&str> {
        self.category_overlay
            .as_ref()
            .and_then(|dir| self.metadata.value.category_for(dir))
    }

    pub fn pad_path(&self, pad: char) -> Option<PathBuf> {
        self.metadata.value.pad(pad).map(LoopWavId::path)
    }

    /// 永続化サブシステム（metadata / track grid / random deck）で直近に発生した
    /// エラー文言を優先順位順に 1 つ返す。
    pub fn persistence_error(&self) -> Option<&String> {
        self.metadata
            .error
            .as_ref()
            .or(self.track_grid_error.as_ref())
            .or(self.random_decks.error.as_ref())
    }

    pub fn pad_file_name(&self, pad: char) -> Option<String> {
        self.pad_path(pad).and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
    }

    pub fn track_grid(&self) -> &[Vec<Option<LoopTrackClip>>] {
        &self.track_grid
    }

    pub fn track_volumes_db(&self) -> &[i32] {
        &self.track_volumes_db
    }

    pub fn track_volume_db(&self, track: usize) -> i32 {
        self.track_volumes_db.get(track).copied().unwrap_or(0)
    }

    pub fn solo_tracks(&self) -> &[bool] {
        &self.solo_tracks
    }

    pub fn cell_label(&self, wav: &LoopWavId) -> String {
        let pad = self
            .metadata
            .value
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
