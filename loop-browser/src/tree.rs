use super::*;

/// `collect_visible` へ渡す展開状態。
///
/// 絞り込み中は `LoopBrowser::expanded` を**書き換えずに**無視し、残ったディレクトリを
/// 全部展開したものとして扱う。解除したときに絞り込み前の折り畳み状態がそのまま戻る。
#[derive(Clone, Copy)]
pub enum ExpandedNodes<'a> {
    Saved(&'a HashSet<NodeKey>),
    All,
}

impl ExpandedNodes<'_> {
    fn contains(&self, key: &NodeKey) -> bool {
        match self {
            Self::Saved(expanded) => expanded.contains(key),
            Self::All => true,
        }
    }
}

impl LoopBrowser {
    pub(crate) fn rebuild_visible(&mut self, selected: Option<&NodeKey>) {
        let filtered = self.filtered_roots();
        let roots = filtered.as_deref().unwrap_or(&self.roots);
        let expanded = match &filtered {
            Some(_) => ExpandedNodes::All,
            None => ExpandedNodes::Saved(&self.expanded),
        };
        let mut visible = Vec::new();
        if self.favorites_only {
            for (anchor, favorite) in self.metadata.value.favorite_dirs.iter().enumerate() {
                if let Some((root_index, root_path, node, components)) =
                    find_favorite_node(roots, favorite)
                {
                    collect_visible(
                        root_index,
                        root_path,
                        node,
                        expanded,
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
            for (root_index, (root_path, root)) in roots.iter().enumerate() {
                collect_visible(
                    root_index,
                    root_path,
                    root,
                    expanded,
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

    pub(crate) fn rebuild_visible_for_path(&mut self, selected: Option<&Path>) {
        self.rebuild_visible(None);
        if let Some(path) = selected {
            if let Some(index) = self.visible.iter().position(|node| node.path == path) {
                self.cursor = index;
            }
        }
    }

    pub fn selected_breadcrumb(&self) -> Vec<String> {
        let Some(node) = self.visible.get(self.cursor) else {
            return Vec::new();
        };
        let Some((root_path, _)) = self.roots.get(node.key.root) else {
            return Vec::new();
        };
        let mut segments = vec![root_path.file_name().map_or_else(
            || root_path.to_string_lossy().into_owned(),
            |name| name.to_string_lossy().into_owned(),
        )];
        let component_count = node
            .key
            .components
            .len()
            .saturating_sub(usize::from(node.is_wav));
        segments.extend(node.key.components.iter().take(component_count).cloned());
        segments
    }

    pub fn selected_direct_category(&self) -> Option<&str> {
        let node = self.visible.get(self.cursor)?;
        let (root_path, _) = self.roots.get(node.key.root)?;
        let component_count = node
            .key
            .components
            .len()
            .saturating_sub(usize::from(node.is_wav));
        let relative = node
            .key
            .components
            .iter()
            .take(component_count)
            .collect::<PathBuf>();
        self.metadata
            .value
            .category_for(&LoopDirId::new(root_path, &relative))
    }
}

pub fn insert_relative_path(root: &mut TreeNode, path: &Path, analysis: LoopWavAnalysis) {
    let components = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();
    insert_components(root, &components, Path::new(""), analysis);
}

fn insert_components(
    node: &mut TreeNode,
    components: &[String],
    parent: &Path,
    analysis: LoopWavAnalysis,
) {
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
                analysis: is_wav.then_some(analysis),
            });
            node.children.len() - 1
        });
    if !rest.is_empty() {
        insert_components(
            &mut node.children[child_index],
            rest,
            &relative_path,
            analysis,
        );
    }
}

pub fn sort_tree(node: &mut TreeNode) {
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
pub fn collect_visible(
    root_index: usize,
    root_path: &Path,
    node: &TreeNode,
    expanded: ExpandedNodes<'_>,
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
    let is_expanded = !node.is_wav && expanded.contains(&key);
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
        analysis: node.analysis,
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

pub fn node_path(root_path: &Path, node: &TreeNode) -> PathBuf {
    root_path.join(&node.relative_path)
}

pub fn find_favorite_node<'a>(
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
