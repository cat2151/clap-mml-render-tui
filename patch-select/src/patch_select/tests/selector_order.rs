use super::*;

/// 一覧は Role=ALL でも各 Role でも Preset 欄の順に並ぶ。Category・patch 名はその内側の順。
#[test]
fn selector_order_follows_the_preset_pane_of_every_role() {
    let mut patches = vec![
        entry("Grand Piano", "Plugin", None),
        entry("Lead Saw", "Plugin", None),
        entry("Sub Bass", "Plugin", None),
        entry("Mystery", "Plugin", None),
        entry("Brass Section", "Plugin", None),
        entry("Bright Piano", "Plugin", None),
        entry("Warm Pad", "Plugin", None),
        entry("Acid Bass", "Plugin", None),
    ];
    let roles = build_role_index(&patches, &[]);
    crate::sort_for_selector(&mut patches, |entry| preset_rank(&roles, entry.display()));
    let names: Vec<_> = patches.iter().map(PatchCatalogEntry::display).collect();
    assert_eq!(names[..2], ["Acid Bass", "Sub Bass"], "{names:?}");
    let prepared = PreparedPresets::build(&patches, &[], &roles).unwrap();
    for role in 0..FilterGroup::ALL.len() {
        let pane = prepared.for_role(role);
        let row_of = |index: &usize| {
            pane.iter()
                .skip(1)
                .filter(|preset| !preset.is_user && !preset.is_favorite)
                .position(|preset| preset.matches.contains(index))
                .unwrap_or(pane.len())
        };
        let rows: Vec<_> = pane[0].matches.iter().map(row_of).collect();
        assert!(rows.is_sorted(), "role {role}: {rows:?} {names:?}");
    }
    let chord = FilterGroup::ALL
        .iter()
        .position(|g| g.role() == Some(PatchRole::Chord))
        .unwrap();
    let chord_rows: Vec<_> = prepared.for_role(chord)[0]
        .matches
        .iter()
        .map(|&index| patches[index].display())
        .collect();
    assert!(chord_rows.len() > 1, "{chord_rows:?}");
}
