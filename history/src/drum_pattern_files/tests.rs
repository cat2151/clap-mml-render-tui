use super::*;

#[test]
fn patterns_are_saved_per_kit_and_removed_when_empty() {
    let _dirs = crate::test_support::temp_local_dirs("drum_pattern_files");
    let kit = "sfz/Kits/909: キット?.sfz";
    assert!(load_drum_pattern_files(kit).unwrap().is_empty());
    save_drum_pattern_file(kit, 3, Some(b"three")).unwrap();
    save_drum_pattern_file(kit, 0, Some(b"zero")).unwrap();
    save_drum_pattern_file("sfz/Kits/other.sfz", 0, Some(b"other")).unwrap();
    // 消すのは指定の pattern だけ。無いファイルを消しても失敗しない。
    save_drum_pattern_file(kit, 7, None).unwrap();
    assert_eq!(
        load_drum_pattern_files(kit).unwrap(),
        [(0, b"zero".to_vec()), (3, b"three".to_vec())]
    );
    save_drum_pattern_file(kit, 0, None).unwrap();
    assert_eq!(
        load_drum_pattern_files(kit).unwrap(),
        [(3, b"three".to_vec())]
    );
    let dir = kit_dir(kit).unwrap();
    assert_eq!(dir.file_name().unwrap(), "sfz_Kits_909_ キット_.sfz");
    assert!(dir.join("pattern_03.mid").exists());
    // 名前の合わないファイルは読まない。
    std::fs::write(dir.join("notes.txt"), "memo").unwrap();
    std::fs::write(dir.join("pattern_x.mid"), "bad").unwrap();
    assert_eq!(load_drum_pattern_files(kit).unwrap().len(), 1);
}

#[test]
fn directory_names_have_no_trailing_dot_or_space_and_are_never_empty() {
    assert_eq!(dir_name("kit. "), "kit");
    assert_eq!(dir_name(""), "_");
    assert_eq!(dir_name("a\tb"), "a_b");
}
