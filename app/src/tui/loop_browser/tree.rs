use super::*;

pub(super) fn insert_relative_path(root: &mut TreeNode, path: &Path, analysis: LoopWavAnalysis) {
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

pub(super) fn sort_tree(node: &mut TreeNode) {
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
pub(super) fn collect_visible(
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

pub(super) fn node_path(root_path: &Path, node: &TreeNode) -> PathBuf {
    root_path.join(&node.relative_path)
}

pub(super) fn find_favorite_node<'a>(
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
