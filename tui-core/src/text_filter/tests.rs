use super::*;

fn matches(condition: &str, fields: &[&str]) -> bool {
    let compiled = compile_condition(condition).expect("valid condition");
    matches_any_field(&compiled, fields)
}

#[test]
fn empty_condition_passes_everything() {
    assert!(matches("", &["anything"]));
    assert!(matches("   ", &["anything"]));
    assert!(matches("", &[]));
}

#[test]
fn whitespace_separated_terms_are_anded() {
    assert!(matches("drum kick", &["drum/kick 01.wav"]));
    assert!(!matches("drum kick", &["drum/snare 01.wav"]));
    assert!(!matches("drum kick", &["perc/kick 01.wav"]));
}

#[test]
fn matching_ignores_case() {
    assert!(matches("KICK", &["drum/kick 01.wav"]));
    assert!(matches("kick", &["Drum/KICK 01.wav"]));
}

#[test]
fn terms_are_regular_expressions() {
    assert!(matches("kick|snare", &["drum/snare 01.wav"]));
    assert!(matches("kick|snare", &["drum/kick 01.wav"]));
    assert!(!matches("kick|snare", &["drum/hat 01.wav"]));
    assert!(matches("^drum", &["drum/kick.wav"]));
    assert!(!matches("^drum", &["perc/drum.wav"]));
    assert!(matches("ki.?ck", &["kick.wav"]));
}

#[test]
fn a_term_may_match_any_of_the_given_fields() {
    let compiled = compile_condition("lead bass").expect("valid");
    // lead は display、bass は category にある。フィールドをまたいだ AND が成立する。
    assert!(matches_any_field(&compiled, &["lead 1.fxp", "bass"]));
    assert!(!matches_any_field(&compiled, &["lead 1.fxp", "pad"]));
    assert!(!matches_any_field(&compiled, &["pad 1.fxp", "bass"]));
}

#[test]
fn no_fields_means_a_non_empty_condition_never_matches() {
    let compiled = compile_condition("kick").expect("valid");
    assert!(!matches_any_field(&compiled, &[]));
}

#[test]
fn invalid_regular_expressions_are_rejected() {
    assert!(compile_condition("[").is_err());
    assert!(compile_condition("kick [").is_err());
    assert!(!is_valid_condition("["));
    assert!(!is_valid_condition("*"));
    assert!(!is_valid_condition("kick ("));
}

#[test]
fn valid_conditions_are_accepted() {
    assert!(is_valid_condition(""));
    assert!(is_valid_condition("kick"));
    assert!(is_valid_condition("kick|snare"));
    assert!(is_valid_condition("drum kick"));
    assert!(is_valid_condition("[ab]"));
}

/// 打鍵の途中で `Err` になる条件は、括弧やクラスの開きっぱなし・先頭の量指定子。
/// **末尾の `|` は `regex` crate では妥当**で、しかも全件にマッチする（空の選択肢が
/// 空文字列にマッチするため）。「入力途中は不正条件」という前提を置かないこと。
#[test]
fn a_trailing_alternation_is_valid_and_matches_everything() {
    assert!(is_valid_condition("kick|"));
    assert!(matches("kick|", &["hat.wav"]));
    assert!(!is_valid_condition("("));
    assert!(!is_valid_condition("[a"));
}
