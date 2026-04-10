use std::collections::HashSet;

pub(super) fn default_lines() -> Vec<String> {
    vec!["cde".to_string()]
}

pub(super) fn merge_patch_phrase_items(dest: &mut Vec<String>, src: Vec<String>) {
    let mut seen = dest.iter().cloned().collect::<HashSet<_>>();
    for item in src {
        if seen.insert(item.clone()) {
            dest.push(item);
        }
    }
}
