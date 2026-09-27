use super::*;

fn state(favorites: &[&str]) -> PatchPhraseState {
    PatchPhraseState {
        history: Vec::new(),
        favorites: favorites.iter().map(|phrase| phrase.to_string()).collect(),
    }
}

fn store(favorite_patches: &[&str], patches: &[(&str, &[&str])]) -> PatchPhraseStore {
    PatchPhraseStore {
        notepad: PatchPhraseState::default(),
        patches: patches
            .iter()
            .map(|(name, favorites)| (name.to_string(), state(favorites)))
            .collect(),
        favorite_patches: favorite_patches
            .iter()
            .map(|name| name.to_string())
            .collect(),
    }
}

#[test]
fn favorite_patch_names_skips_patches_without_phrase_favorites() {
    let store = store(
        &["A.fxp", "B.fxp", "C.fxp"],
        &[("A.fxp", &["c"]), ("B.fxp", &[])],
    );

    assert_eq!(favorite_patch_names(&store), ["A.fxp"]);
}

#[test]
fn favorite_patch_names_keeps_the_newest_first_order() {
    let store = store(
        &["C.fxp", "A.fxp", "B.fxp"],
        &[("A.fxp", &["c"]), ("B.fxp", &["d"]), ("C.fxp", &["e"])],
    );

    assert_eq!(favorite_patch_names(&store), ["C.fxp", "A.fxp", "B.fxp"]);
}

#[test]
fn favorite_patch_names_removes_duplicates() {
    let store = store(
        &["A.fxp", "B.fxp", "A.fxp"],
        &[("A.fxp", &["c"]), ("B.fxp", &["d"])],
    );

    assert_eq!(favorite_patch_names(&store), ["A.fxp", "B.fxp"]);
}
