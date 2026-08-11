use super::DrumRole;

#[test]
fn all_lists_every_role_once() {
    for (index, role) in DrumRole::ALL.iter().enumerate() {
        assert!(
            !DrumRole::ALL[index + 1..].contains(role),
            "{}",
            role.label()
        );
    }
}

/// NOTE 欄は4桁ぶんしか無い。溢れるとラベルが切れて役割が読めなくなる。
#[test]
fn labels_are_short_enough_for_the_note_column() {
    for role in DrumRole::ALL {
        assert!(role.label().len() <= 4, "{}", role.label());
        assert!(role.label().is_ascii());
    }
}
