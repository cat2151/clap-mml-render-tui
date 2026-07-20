use crossterm::event::KeyCode;
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::loop_browser_metadata::{category_keys, metadata_path, LoopBrowserMetadata, LoopDirId};

mod input;
mod playback;

const FAVORITE_REMOVED_NOTICE_DURATION: Duration = Duration::from_millis(1_500);

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

pub(crate) struct LoopBrowser {
    roots: Vec<(PathBuf, TreeNode)>,
    expanded: HashSet<NodeKey>,
    pub(super) visible: Vec<VisibleLoopNode>,
    pub(super) cursor: usize,
    pub(super) list_state: ListState,
    pub(super) page_size: usize,
    pub(super) error: Option<String>,
    pub(super) metadata_error: Option<String>,
    metadata: LoopBrowserMetadata,
    metadata_path: Option<PathBuf>,
    metadata_writable: bool,
    pub(super) favorites_only: bool,
    pub(super) category_overlay: Option<LoopDirId>,
    pub(super) category_keys: Vec<(char, String)>,
    pub(super) notice: Option<LoopBrowserNotice>,
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
            metadata_error: None,
            metadata: LoopBrowserMetadata::default(),
            metadata_path: None,
            metadata_writable: true,
            favorites_only: false,
            category_overlay: None,
            category_keys: Vec::new(),
            notice: None,
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
        *self = match crate::loop_library::load_index(cfg) {
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
            .map(|node| LoopBrowserAction::Play(node.path.clone()))
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
    metadata: &LoopBrowserMetadata,
    category_keys: &[(char, String)],
    components: Vec<String>,
    anchor: Option<usize>,
    depth: usize,
    display_name: Option<String>,
    output: &mut Vec<VisibleLoopNode>,
) {
    let key = NodeKey {
        root: root_index,
        components: components.clone(),
        anchor,
    };
    let is_expanded = expanded.contains(&key);
    let dir_id = (!node.is_wav).then(|| LoopDirId::new(root_path, &node.relative_path));
    let favorite = dir_id.as_ref().is_some_and(|dir| metadata.is_favorite(dir));
    let category = dir_id.as_ref().and_then(|dir| {
        metadata.category_for(dir).and_then(|category| {
            category_keys
                .iter()
                .any(|(_, configured)| configured == category)
                .then(|| category.to_string())
        })
    });
    output.push(VisibleLoopNode {
        key: key.clone(),
        depth,
        name: display_name.unwrap_or_else(|| node.name.clone()),
        is_wav: node.is_wav,
        expanded: is_expanded,
        path: node_path(root_path, node),
        favorite,
        category,
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
                metadata,
                category_keys,
                child_components,
                anchor,
                depth + 1,
                None,
                output,
            );
        }
    }
}

fn node_path(root_path: &Path, node: &TreeNode) -> PathBuf {
    root_path.join(&node.relative_path)
}

fn find_favorite_node<'a>(
    roots: &'a [(PathBuf, TreeNode)],
    favorite: &LoopDirId,
) -> Option<(usize, &'a Path, &'a TreeNode, Vec<String>)> {
    for (root_index, (root_path, root)) in roots.iter().enumerate() {
        if let Some((node, components)) = find_node(root_path, root, favorite, Vec::new()) {
            return Some((root_index, root_path.as_path(), node, components));
        }
    }
    None
}

fn find_node<'a>(
    root_path: &Path,
    node: &'a TreeNode,
    target: &LoopDirId,
    components: Vec<String>,
) -> Option<(&'a TreeNode, Vec<String>)> {
    if !node.is_wav && LoopDirId::new(root_path, &node.relative_path).matches(target) {
        return Some((node, components));
    }
    for child in &node.children {
        if child.is_wav {
            continue;
        }
        let mut child_components = components.clone();
        child_components.push(child.name.clone());
        if let Some(found) = find_node(root_path, child, target, child_components) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests;
