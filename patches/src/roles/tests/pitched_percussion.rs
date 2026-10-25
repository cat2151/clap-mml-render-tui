use super::*;

#[test]
fn explicit_measured_category_moves_perc_to_lead_without_shadowing_triggered() {
    let index = PatchRoleIndex::build(
        [
            input("bank/01 percsyn", Some("pitched percussion")),
            input("bank/02 clap", None),
            input("bank/03 perc sequence", Some("pitched percussion")),
        ],
        &[],
    );
    assert_eq!(index.role_of("bank/01 percsyn"), Some(PatchRole::Lead));
    assert_eq!(
        index.preset_label_of("bank/01 percsyn"),
        Some("pitched percussion")
    );
    assert_eq!(index.drum_role_of("bank/01 percsyn"), None);
    assert_eq!(
        index.drum_candidates(DrumPatchRole::Percussion),
        ["bank/02 clap"]
    );
    assert_eq!(
        index.role_of("bank/03 perc sequence"),
        Some(PatchRole::Triggered)
    );
}
