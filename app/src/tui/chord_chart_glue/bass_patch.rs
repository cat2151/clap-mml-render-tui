//! Chord Chart Bass preview 用 patch の非ブロッキング解決。

use crate::tui::PatchLoadState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::tui) enum BassPatchResolution {
    Ready(String),
    Loading,
    CatalogError(String),
    NoCandidate,
}

pub(super) fn resolve(
    saved_patch: Option<&str>,
    patch_load: &PatchLoadState,
) -> BassPatchResolution {
    if let Some(patch) = saved_patch {
        return BassPatchResolution::Ready(patch.to_owned());
    }

    match patch_load {
        PatchLoadState::Loading => BassPatchResolution::Loading,
        PatchLoadState::Err(error) => BassPatchResolution::CatalogError(error.clone()),
        PatchLoadState::Ready(snapshot) => snapshot
            .patch_roles()
            .candidates(cmrt_patches::PatchRole::Bass)
            .first()
            .cloned()
            .map_or(BassPatchResolution::NoCandidate, BassPatchResolution::Ready),
    }
}

#[cfg(test)]
mod tests;
