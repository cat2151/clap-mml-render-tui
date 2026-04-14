pub(super) use std::collections::HashSet;

pub(super) use super::*;

#[path = "patch_tests/generate.rs"]
mod generate;
#[path = "patch_tests/random_patch_rerender.rs"]
mod random_patch_rerender;
#[path = "patch_tests/random_patch_selection.rs"]
mod random_patch_selection;
