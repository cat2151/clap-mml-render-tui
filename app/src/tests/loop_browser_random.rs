use super::*;
use std::collections::HashSet;

fn wav(relative: &str) -> LoopWavId {
    LoopWavId::new(Path::new("/loops"), Path::new(relative))
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!(
            "cmrt-random-deck-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join("loop_browser")
        .join("random_decks.toml")
}

#[test]
fn deck_draws_each_candidate_once_and_avoids_cycle_boundary_repeat() {
    let candidates = vec![wav("a.wav"), wav("b.wav"), wav("c.wav")];
    let mut state = LoopRandomDeckState::default();
    let mut selected = Vec::new();
    let mut current = None;

    for _ in 0..candidates.len() {
        let next = state
            .draw(LoopRandomScope::All, &candidates, current.as_ref())
            .unwrap();
        current = Some(next.clone());
        selected.push(next.relative);
    }

    assert_eq!(
        selected.iter().collect::<HashSet<_>>().len(),
        candidates.len()
    );
    let previous = current.unwrap();
    let next = state
        .draw(LoopRandomScope::All, &candidates, Some(&previous))
        .unwrap();
    assert!(!next.matches(&previous));
}

#[test]
fn candidate_changes_rebuild_only_the_requested_scope() {
    let original = vec![wav("a.wav"), wav("b.wav")];
    let changed = vec![wav("c.wav")];
    let favorite = LoopRandomScope::Favorites;
    let mut state = LoopRandomDeckState::default();
    let all_first = state.draw(LoopRandomScope::All, &original, None).unwrap();
    state.draw(favorite.clone(), &original, None).unwrap();

    let favorite_next = state.draw(favorite, &changed, None).unwrap();
    assert!(favorite_next.matches(&changed[0]));

    let all_next = state.draw(LoopRandomScope::All, &original, None).unwrap();
    assert!(!all_next.matches(&all_first));
}

#[test]
fn state_round_trips_and_continues_the_saved_deck() {
    let path = temp_path("round-trip");
    let candidates = vec![wav("a.wav"), wav("b.wav"), wav("c.wav")];
    let mut state = LoopRandomDeckState::default();
    let first = state.draw(LoopRandomScope::All, &candidates, None).unwrap();

    save_to(&path, &state).unwrap();
    let mut loaded = load_from(&path).unwrap();
    assert_eq!(loaded, state);
    let second = loaded
        .draw(LoopRandomScope::All, &candidates, Some(&first))
        .unwrap();
    let third = loaded
        .draw(LoopRandomScope::All, &candidates, Some(&second))
        .unwrap();

    assert!(!first.matches(&second));
    assert!(!first.matches(&third));
    assert!(!second.matches(&third));
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn corrupt_or_unsupported_state_is_rejected_and_can_be_replaced() {
    let path = temp_path("corrupt");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "version = 999\n").unwrap();

    assert!(load_from(&path)
        .unwrap_err()
        .to_string()
        .contains("versionが一致しません"));

    save_to(&path, &LoopRandomDeckState::default()).unwrap();
    assert_eq!(load_from(&path).unwrap(), LoopRandomDeckState::default());
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[test]
fn invalid_next_position_and_duplicate_wavs_are_rejected() {
    let path = temp_path("invalid");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"version = 1

[[decks]]
next = 3
[decks.scope]
kind = "all"
[[decks.order]]
root = "/loops"
relative = "a.wav"
[[decks.order]]
root = "/loops"
relative = "a.wav"
"#,
    )
    .unwrap();

    assert!(load_from(&path).is_err());

    std::fs::write(
        &path,
        r#"version = 1

[[decks]]
next = 2
[decks.scope]
kind = "all"
[[decks.order]]
root = "/loops"
relative = "a.wav"
[[decks.order]]
root = "/loops"
relative = "a.wav"
"#,
    )
    .unwrap();
    assert!(load_from(&path)
        .unwrap_err()
        .to_string()
        .contains("重複したWAV"));
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}
