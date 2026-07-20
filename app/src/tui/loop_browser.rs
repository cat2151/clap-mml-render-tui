use crossterm::event::KeyCode;
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{Mode, PlayState, TuiApp};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NodeKey {
    root: usize,
    components: Vec<String>,
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
}

pub(crate) struct LoopBrowser {
    roots: Vec<(PathBuf, TreeNode)>,
    expanded: HashSet<NodeKey>,
    pub(super) visible: Vec<VisibleLoopNode>,
    pub(super) cursor: usize,
    pub(super) list_state: ListState,
    pub(super) page_size: usize,
    pub(super) error: Option<String>,
}

pub(super) enum LoopBrowserAction {
    Continue,
    Play(PathBuf),
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
            page_size: 1,
            error: None,
        }
    }
}

impl LoopBrowser {
    pub(super) fn reload(&mut self, cfg: &crate::config::Config) {
        *self = match crate::loop_library::load_index(cfg) {
            Ok(index) => Self::from_index(index),
            Err(error) => Self {
                error: Some(format!("{error}\ncmrt scan-loops を実行してください")),
                ..Self::default()
            },
        };
    }

    fn from_index(index: crate::loop_library::LoopIndex) -> Self {
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
            });
            roots.push((root_path, root));
        }
        let mut browser = Self {
            roots,
            expanded,
            ..Self::default()
        };
        browser.rebuild_visible(None);
        browser
    }

    pub(super) fn handle_key(&mut self, key: KeyCode) -> LoopBrowserAction {
        match key {
            KeyCode::Esc => LoopBrowserAction::Return,
            KeyCode::Char('q') => LoopBrowserAction::Quit,
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor(-1),
            KeyCode::PageDown => self.move_cursor(self.page_size as isize),
            KeyCode::PageUp => self.move_cursor(-(self.page_size as isize)),
            KeyCode::Char('l') | KeyCode::Right => self.expand_or_play(),
            KeyCode::Char('h') | KeyCode::Left => {
                self.collapse_or_select_parent();
                LoopBrowserAction::Continue
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.selected_play_action(),
            _ => LoopBrowserAction::Continue,
        }
    }

    fn move_cursor(&mut self, delta: isize) -> LoopBrowserAction {
        if self.visible.is_empty() {
            return LoopBrowserAction::Continue;
        }
        let max = self.visible.len().saturating_sub(1) as isize;
        let next = (self.cursor as isize + delta).clamp(0, max) as usize;
        if next == self.cursor {
            return LoopBrowserAction::Continue;
        }
        self.cursor = next;
        self.list_state.select(Some(next));
        self.selected_play_action()
    }

    fn selected_play_action(&self) -> LoopBrowserAction {
        self.visible
            .get(self.cursor)
            .filter(|node| node.is_wav)
            .map(|node| LoopBrowserAction::Play(node.path.clone()))
            .unwrap_or(LoopBrowserAction::Continue)
    }

    fn expand_or_play(&mut self) -> LoopBrowserAction {
        let Some(node) = self.visible.get(self.cursor).cloned() else {
            return LoopBrowserAction::Continue;
        };
        if node.is_wav {
            return LoopBrowserAction::Play(node.path);
        }
        if self.expanded.insert(node.key.clone()) {
            self.rebuild_visible(Some(&node.key));
        }
        LoopBrowserAction::Continue
    }

    fn collapse_or_select_parent(&mut self) {
        let Some(node) = self.visible.get(self.cursor).cloned() else {
            return;
        };
        if !node.is_wav && self.expanded.remove(&node.key) {
            self.rebuild_visible(Some(&node.key));
            return;
        }
        if node.key.components.is_empty() {
            return;
        }
        let mut parent = node.key;
        parent.components.pop();
        self.rebuild_visible(Some(&parent));
    }

    fn rebuild_visible(&mut self, selected: Option<&NodeKey>) {
        let mut visible = Vec::new();
        for (root_index, (root_path, root)) in self.roots.iter().enumerate() {
            collect_visible(
                root_index,
                root_path,
                root,
                &self.expanded,
                Vec::new(),
                0,
                &mut visible,
            );
        }
        self.visible = visible;
        self.cursor = selected
            .and_then(|key| self.visible.iter().position(|node| &node.key == key))
            .unwrap_or_else(|| self.cursor.min(self.visible.len().saturating_sub(1)));
        self.list_state
            .select((!self.visible.is_empty()).then_some(self.cursor));
    }
}

fn insert_relative_path(root: &mut TreeNode, path: &Path) {
    let components = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();
    insert_components(root, &components, Path::new(""));
}

fn insert_components(node: &mut TreeNode, components: &[String], parent: &Path) {
    let Some((name, rest)) = components.split_first() else {
        return;
    };
    let relative_path = parent.join(name);
    let is_wav = rest.is_empty();
    let child_index = node
        .children
        .iter()
        .position(|child| child.name == *name)
        .unwrap_or_else(|| {
            node.children.push(TreeNode {
                name: name.clone(),
                relative_path: relative_path.clone(),
                children: Vec::new(),
                is_wav,
            });
            node.children.len() - 1
        });
    if !rest.is_empty() {
        insert_components(&mut node.children[child_index], rest, &relative_path);
    }
}

fn sort_tree(node: &mut TreeNode) {
    node.children.sort_by(|left, right| {
        left.is_wav
            .cmp(&right.is_wav)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    });
    for child in &mut node.children {
        sort_tree(child);
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_visible(
    root_index: usize,
    root_path: &Path,
    node: &TreeNode,
    expanded: &HashSet<NodeKey>,
    components: Vec<String>,
    depth: usize,
    output: &mut Vec<VisibleLoopNode>,
) {
    let key = NodeKey {
        root: root_index,
        components: components.clone(),
    };
    let is_expanded = expanded.contains(&key);
    output.push(VisibleLoopNode {
        key: key.clone(),
        depth,
        name: node.name.clone(),
        is_wav: node.is_wav,
        expanded: is_expanded,
        path: root_path.join(&node.relative_path),
    });
    if !node.is_wav && is_expanded {
        for child in &node.children {
            let mut child_components = components.clone();
            child_components.push(child.name.clone());
            collect_visible(
                root_index,
                root_path,
                child,
                expanded,
                child_components,
                depth + 1,
                output,
            );
        }
    }
}

impl<'a> TuiApp<'a> {
    pub(super) fn start_loop_browser(&mut self) {
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
        self.loop_browser.reload(&self.cfg);
        self.mode = Mode::LoopBrowser;
    }

    pub(super) fn finish_loop_browser(&mut self) {
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
        self.mode = Mode::Normal;
    }

    pub(super) fn play_loop_file(&self, path: PathBuf) {
        let session = self.begin_playback_session();
        let display = path.to_string_lossy().into_owned();
        self.set_play_state_if_current(session, PlayState::Playing(display.clone()));
        let state = Arc::clone(&self.play_state);
        let playback_session = Arc::clone(&self.playback_session);
        let active_sink = Arc::clone(&self.active_sink);
        std::thread::spawn(move || {
            let result = play_file_for_session(&path, session, &playback_session, &active_sink);
            if let Err(error) = result {
                TuiApp::clear_active_sink_for_session(&active_sink, &playback_session, session);
                TuiApp::set_play_state_for_session(
                    &state,
                    &playback_session,
                    session,
                    PlayState::Err(format!("WAV再生に失敗: {error}")),
                );
            } else {
                TuiApp::clear_active_sink_for_session(&active_sink, &playback_session, session);
                TuiApp::set_play_state_for_session(
                    &state,
                    &playback_session,
                    session,
                    PlayState::Done(display),
                );
            }
        });
    }
}

fn play_file_for_session(
    path: &Path,
    session: u64,
    playback_session: &std::sync::atomic::AtomicU64,
    active_sink: &std::sync::Mutex<Option<Arc<rodio::Sink>>>,
) -> anyhow::Result<()> {
    let file = std::fs::File::open(path)?;
    let source = rodio::Decoder::new(std::io::BufReader::new(file))?;
    let (_stream, stream_handle) = rodio::OutputStream::try_default()?;
    let sink = Arc::new(rodio::Sink::try_new(&stream_handle)?);
    if !TuiApp::playback_session_is_current(playback_session, session) {
        return Ok(());
    }
    {
        let mut guard = active_sink.lock().unwrap();
        if !TuiApp::playback_session_is_current(playback_session, session) {
            return Ok(());
        }
        *guard = Some(Arc::clone(&sink));
    }
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loop_library::{LoopIndex, LoopRootIndex};

    fn browser() -> LoopBrowser {
        LoopBrowser::from_index(LoopIndex {
            version: 1,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: vec![
                    "Pack/Bass/B.wav".to_string(),
                    "Pack/Bass/a.wav".to_string(),
                    "Pack/Drums/Kick.wav".to_string(),
                ],
            }],
        })
    }

    #[test]
    fn root_is_expanded_and_directories_sort_before_wavs() {
        let browser = browser();
        assert_eq!(browser.visible.len(), 2);
        assert_eq!(browser.visible[0].name, "/loops");
        assert_eq!(browser.visible[1].name, "Pack");
    }

    #[test]
    fn hjkl_expand_navigate_play_and_select_parent() {
        let mut browser = browser();
        assert!(matches!(
            browser.handle_key(KeyCode::Char('j')),
            LoopBrowserAction::Continue
        ));
        browser.handle_key(KeyCode::Char('l'));
        assert_eq!(browser.visible.len(), 4);
        browser.handle_key(KeyCode::Char('j'));
        browser.handle_key(KeyCode::Char('l'));
        assert_eq!(browser.visible[3].name, "a.wav");
        assert!(matches!(
            browser.handle_key(KeyCode::Char('j')),
            LoopBrowserAction::Play(path) if path.ends_with("a.wav")
        ));
        browser.handle_key(KeyCode::Char('h'));
        assert_eq!(browser.visible[browser.cursor].name, "Bass");
    }
}
