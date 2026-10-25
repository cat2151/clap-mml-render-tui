use super::*;

#[test]
fn measured_category_is_searchable_in_lead_and_absent_from_drum() {
    let patches = vec![
        entry("01 PERCSYN", "Dexed", Some("pitched percussion")),
        entry("02 CLAP", "Dexed", None),
    ];
    let roles = build_role_index(&patches, &[]);
    let prepared = PreparedPresets::build(&patches, &[], &roles).unwrap();
    let lead = FilterGroup::ALL
        .iter()
        .position(|g| g.role() == Some(PatchRole::Lead))
        .unwrap();
    let drum = FilterGroup::ALL
        .iter()
        .position(|g| g.role() == Some(PatchRole::Drum))
        .unwrap();
    let pitched = prepared
        .for_role(lead)
        .iter()
        .find(|p| p.label == "pitched percussion")
        .unwrap();
    assert_eq!(&*pitched.matches, [0]);
    assert_eq!(&*prepared.for_role(drum)[0].matches, [1]);
}
