//! Floe の `.floe-preset` patch 体系。
//!
//! preset root 直下の先頭ディレクトリをカテゴリとし、その下の相対パスは
//! 共通の natural sort へそのまま渡す。

const SORT_PRIORITY: u8 = 0;
const FLOE_PRESET_EXTENSION: &str = ".floe-preset";
const PATH_SEPARATORS: [char; 2] = ['/', '\\'];

pub(crate) fn has_floe_preset_extension(path: &str) -> bool {
    path.split(PATH_SEPARATORS).any(|component| {
        component.len() > FLOE_PRESET_EXTENSION.len()
            && component
                .get(component.len() - FLOE_PRESET_EXTENSION.len()..)
                .is_some_and(|suffix| suffix.eq_ignore_ascii_case(FLOE_PRESET_EXTENSION))
    })
}

pub(crate) fn category_sort_parts(path: &str) -> (&str, u8, &str, &str) {
    let (category, rest) = crate::layout::split_first_path_segment(path);
    (category, SORT_PRIORITY, "", rest)
}

pub(crate) fn path_sort_parts(path: &str) -> (u8, &str) {
    (SORT_PRIORITY, path)
}

#[cfg(test)]
mod tests;
