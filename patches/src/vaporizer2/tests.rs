use super::layout::{category_name_for_code, PATCH_CATEGORY_CODES};
use crate::layout::{patch_category, patch_matches_categories, PatchLayout};
use crate::{group_patch_pairs_by_category, PatchCategory};

/// 実プリセットの名前の形。**カテゴリはファイル名先頭 2 文字を展開した名前**になる。
#[test]
fn the_first_two_letters_of_the_file_name_are_the_category() {
    assert_eq!(
        PatchLayout::of("AR Accent Arp.vvp"),
        PatchLayout::Vaporizer2
    );
    assert_eq!(patch_category("AR Accent Arp.vvp"), "Arpeggio");
    assert_eq!(patch_category("PD Emily.vvp"), "Pad");
    assert_eq!(patch_category("BA Sub Bass.vvp"), "Bass");
}

/// サブディレクトリへ置いたユーザープリセットでも、見るのは**ファイル名**の先頭 2 文字。
/// ディレクトリ名を先頭セグメントとして読む cartridge の規則をここへ持ち込まない。
#[test]
fn a_preset_in_a_subdirectory_still_reads_its_file_name() {
    assert_eq!(patch_category("MyBank/LD Screamer.vvp"), "Lead");
    assert_eq!(patch_category("MyBank/Deep/SY Warm.vvp"), "Synth");
}

/// 表に無いコードは生の 2 文字のまま出す。**カテゴリを失わせない**のが狙いで、
/// ユーザーが独自のコードで保存していればそのコードでまとまって見える。
#[test]
fn an_unknown_code_is_shown_as_is() {
    assert_eq!(patch_category("ZZ Homemade.vvp"), "ZZ");
    assert_eq!(patch_category("x.vvp"), "x");
}

/// 照合用に小文字化された patch 文字列でも同じ展開名を返す。
///
/// **ここが崩れるとカテゴリが 2 つに割れる。** グループ化はキーを小文字側から、
/// 表示名を display 側から作る（`crate::grouping`）ので、両方が同じ文字列に
/// ならないと同じカテゴリが別の見出しで 2 回出る。
#[test]
fn the_lowercased_form_lands_in_the_same_category() {
    assert_eq!(patch_category("ar accent arp.vvp"), "Arpeggio");
    assert_eq!(category_name_for_code("ar"), Some("Arpeggio"));

    let pairs = vec![
        (
            "AR Accent Arp.vvp".to_string(),
            "ar accent arp.vvp".to_string(),
        ),
        ("AR Comber.vvp".to_string(), "ar comber.vvp".to_string()),
    ];
    assert_eq!(
        group_patch_pairs_by_category(&pairs),
        vec![PatchCategory {
            name: "Arpeggio".to_string(),
            patches: vec!["AR Accent Arp.vvp".to_string(), "AR Comber.vvp".to_string()],
        }]
    );
}

/// 用途別の絞り込み（`crate::selection`）は展開名で照合する。
/// **2 文字コードでは当たらない**ので、config には展開名を書いてもらう。
#[test]
fn the_role_filters_match_on_the_expanded_name() {
    let expanded = vec!["Arpeggio".to_string()];
    assert!(patch_matches_categories("AR Accent Arp.vvp", &expanded));

    let code = vec!["AR".to_string()];
    assert!(!patch_matches_categories("AR Accent Arp.vvp", &code));
}

/// 既定カテゴリ名は必ず表の展開名であること。**綴りを 1 文字間違えると候補が
/// 黙って 0 件になる**ので、定数そのものを表と突き合わせる。
#[test]
fn every_default_category_name_exists_in_the_code_table() {
    let names: Vec<&str> = PATCH_CATEGORY_CODES.iter().map(|(_, name)| *name).collect();
    let defaults = [
        super::DEFAULT_CHORD_PATCH_CATEGORY_NAMES.as_slice(),
        super::DEFAULT_BASS_PATCH_CATEGORY_NAMES.as_slice(),
        super::DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES.as_slice(),
        super::DEFAULT_DRUM_PATCH_CATEGORY_NAMES.as_slice(),
    ];
    for name in defaults.iter().flat_map(|group| group.iter()) {
        assert!(names.contains(name), "表に無い既定カテゴリ名: {name}");
    }
}

/// コードは 2 文字ちょうど・重複無し・展開名も重複無し。
/// 展開名が重複すると別コードの音色が同じ見出しへ混ざる。
#[test]
fn the_code_table_has_no_duplicates() {
    let mut codes: Vec<&str> = PATCH_CATEGORY_CODES.iter().map(|(code, _)| *code).collect();
    let count = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), count);

    let mut names: Vec<&str> = PATCH_CATEGORY_CODES.iter().map(|(_, name)| *name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count);

    assert!(PATCH_CATEGORY_CODES
        .iter()
        .all(|(code, _)| code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase())));
}

/// 拡張子の判定は大文字小文字を問わず、どのコンポーネントに現れてもよい
/// （play server 側の `is_vvp_patch_path` と同じ規則）。
#[test]
fn the_extension_decides_regardless_of_case() {
    assert_eq!(
        PatchLayout::of("User/PD Emily.VVP"),
        PatchLayout::Vaporizer2
    );
    assert_eq!(
        PatchLayout::of("User\\PD Emily.vvp"),
        PatchLayout::Vaporizer2
    );
    // stem の無い `.vvp` は音色ではない。
    assert_eq!(PatchLayout::of(".vvp"), PatchLayout::Cartridge);
    assert_eq!(PatchLayout::of("PD Emily.vvpx"), PatchLayout::Cartridge);
}

/// マルチバイト名でも先頭 2 **文字**で切る（バイトで切ると panic する）。
#[test]
fn a_multibyte_name_is_cut_by_characters() {
    assert_eq!(patch_category("ぱっど.vvp"), "ぱっ");
}

/// 実際にインストールされている 460 件の `.vvp` を全部カテゴリ分けしてみる。
///
/// **コード表が実データと食い違っていないかは、これでしか分からない。** 表に無い
/// コードは生の 2 文字で出てしまい、画面には出るぶん**気づけない**（候補から静かに
/// 外れるだけ）ので、ここで落とす。
///
/// 音色置き場は個人のパスなのでコードに書かず、`CMRT_TEST_VAPORIZER2_PRESETS` で渡す。
/// 未設定なら skip（`cargo test` の通常実行では走らない `#[ignore]`）。
/// 列挙そのものは play server の責務だが、**このテストが確かめたいのは
/// 「表 対 実データ」**で、表を持っているのはこの crate なのでここに置く。
#[test]
#[ignore = "実際にインストールされた Vaporizer2 のプリセットが要る"]
fn every_installed_preset_lands_in_a_known_category() {
    let Some(dir) = std::env::var_os("CMRT_TEST_VAPORIZER2_PRESETS") else {
        panic!("CMRT_TEST_VAPORIZER2_PRESETS に音色置き場を渡すこと");
    };

    let mut names: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("音色置き場を読めない") {
        let path = entry.expect("dir entry").path();
        let name = path
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .into_owned();
        if PatchLayout::of(&name) == PatchLayout::Vaporizer2 {
            names.push(name);
        }
    }
    assert!(!names.is_empty(), "`.vvp` が 1 件も無い: {dir:?}");

    let known: Vec<&str> = PATCH_CATEGORY_CODES.iter().map(|(_, name)| *name).collect();
    let unknown: Vec<&String> = names
        .iter()
        .filter(|name| !known.contains(&patch_category(name)))
        .collect();
    assert!(unknown.is_empty(), "コード表に無いカテゴリ: {unknown:?}");

    // 用途別の既定が実データで何件拾うか。ここが 0 件だと画面で「音色が出ない」になる。
    // 出た数は Stage 6（カタログ配線）以降のベースラインになるので print しておく。
    let count_for = |defaults: &[&str]| -> usize {
        let categories: Vec<String> = defaults.iter().map(|name| name.to_string()).collect();
        names
            .iter()
            .filter(|name| patch_matches_categories(&name.to_lowercase(), &categories))
            .count()
    };
    let chord = count_for(&super::DEFAULT_CHORD_PATCH_CATEGORY_NAMES);
    let bass = count_for(&super::DEFAULT_BASS_PATCH_CATEGORY_NAMES);
    let arpeggio = count_for(&super::DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES);
    let drum = count_for(&super::DEFAULT_DRUM_PATCH_CATEGORY_NAMES);
    eprintln!(
        "`.vvp` {} 件 — chord {chord} / bass {bass} / arpeggio {arpeggio} / drum {drum}",
        names.len()
    );

    // chord はさらに mono を外す（Stage 7）ので、ここでの数は上限。
    assert!(chord > 0, "chord 行の既定カテゴリが 1 件も拾えていない");
    assert!(bass > 0, "bass 行の既定カテゴリが 1 件も拾えていない");
    assert!(
        arpeggio > 0,
        "arpeggio 行の既定カテゴリが 1 件も拾えていない"
    );
    // drum は実データに `Drum` が 9 件しか無く、`Drum kit` は 0 件。
    // 「ほぼ空でよい」と決めてあるので数は問わない（`defaults.rs` の doc）。
}
