use super::*;

fn section(degrees: &str) -> Section {
    Section::new(SectionId::new(1), "A", degrees)
}

/// degrees は**打った文字列そのまま**。解釈も検証も切り詰めもしない。
#[test]
fn the_degrees_keep_whatever_was_put_into_them() {
    for degrees in ["I-V-VIm-IV", "C-7", "zzz", "", "   ", "I-V | VIm-IV"] {
        assert_eq!(section(degrees).degrees, degrees);
    }
}

/// 同じ section を何度並べても、参照している実体は 1 つ。
#[test]
fn the_arrangement_resolves_every_entry_through_the_section_id() {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    let b = song.push_section("B", "IIm-V-I");
    song.arrangement = vec![a, a, b];

    let names: Vec<&str> = song
        .arranged_sections()
        .map(|section| section.name.as_str())
        .collect();
    assert_eq!(names, vec!["A", "A", "B"]);
}

/// 参照先の無い id は、集計でも走査でも黙って飛ばす（panic させない）。
#[test]
fn an_arrangement_entry_pointing_at_a_missing_section_is_skipped() {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    song.arrangement = vec![a, SectionId::new(999)];

    assert_eq!(song.arranged_sections().count(), 1);
    assert!(song.section(SectionId::new(999)).is_none());
}

/// 初期値は**空**。既定の曲（＝コード進行のハードコード）は持たない。
/// 中身はカタログから引くか手で打つかのどちらかでしか入らない。
#[test]
fn the_initial_song_is_empty_and_carries_no_progression_of_its_own() {
    let song = Song::empty();
    assert!(song.sections.is_empty());
    assert!(song.arrangement.is_empty());
    // Key / BPM は別フィールドではなく、chord2mml の書式 1 本として持つ。
    // ここだけは既定値を持つ（コード進行ではないので）。
    assert_eq!(song.prefix, "Key=C BPM120");
    // `Default` も同じもの。既定の曲を返す入口はもう無い。
    assert_eq!(Song::default(), song);
}

#[test]
fn issued_section_ids_never_collide() {
    let mut song = Song::empty();
    let zeroth = song.push_section("A", "I-IV");
    let first = song.push_section("B", "I-IV");
    let second = song.push_section("C", "I-IV");
    let existing: Vec<SectionId> = song.sections.iter().map(|section| section.id).collect();
    assert_eq!(existing, vec![zeroth, first, second]);
    assert_ne!(first, second);

    // 手で id を差し込んだ曲でも、reseed 後は衝突しない。
    song.sections
        .push(Section::new(SectionId::new(50), "D", "I-IV"));
    song.reseed_section_ids();
    let next = song.issue_section_id();
    assert_eq!(next, SectionId::new(51));
    assert!(song.section(next).is_none());
}

/// prefix は**何を入れても丸められない**。クランプも綴りの照合もしない。
#[test]
fn the_prefix_keeps_whatever_was_put_into_it() {
    let mut song = Song::empty();

    for text in ["Key=A BPM120", "Key:H BPM:0", "", "まったくの自由文"] {
        song.prefix = text.to_string();
        assert_eq!(song.prefix, text);
    }
}
