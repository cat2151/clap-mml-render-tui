use super::*;

/// 配布中の `chord-progressions.json` のスナップショット。ネットワーク無しで
/// 「実際のカタログが変換できるか」を検証するために同梱している。
const CATALOG_JSON: &str = include_str!("../../testdata/chord-progressions.json");

fn catalog() -> ChordProgressionCatalog {
    ChordProgressionCatalog::from_json(CATALOG_JSON).unwrap()
}

#[test]
fn catalog_parses_the_shipped_json() {
    let catalog = catalog();
    assert_eq!(catalog.len(), 60);
    assert_eq!(catalog.entries()[0].degrees, "I-IV-V-I");
    assert_eq!(catalog.entries()[0].chord_count(), 4);
}

#[test]
fn empty_or_malformed_json_is_rejected() {
    assert!(ChordProgressionCatalog::from_json("[]").is_err());
    assert!(ChordProgressionCatalog::from_json("{}").is_err());
    assert!(ChordProgressionCatalog::from_json(r#"[{"degrees": ""}]"#).is_err());
}

#[test]
fn key_prefix_transposes_degrees() {
    assert_eq!(chord_notes("I", "C").unwrap(), vec![vec![60, 64, 67]]);
    assert_eq!(chord_notes("I", "D").unwrap(), vec![vec![62, 66, 69]]);
    assert_eq!(
        chord_notes("I-IV-V-I", "C").unwrap().len(),
        4,
        "ハイフン区切りの4コードは4和音になる"
    );
}

/// ハイフンは chord2mml が受け付ける区切り文字なので、スペース区切りと同じ結果になる。
/// （`chord_notes` は degrees をハイフンで数えるので、比較相手は変換だけを通す）
#[test]
fn hyphen_and_space_separators_agree() {
    for entry in catalog().entries() {
        let spaced = format!("Key:F# {}", entry.degrees.replace('-', " "));
        let expected = crate::mml_note_progression(&chord2mml_core::convert(&spaced).unwrap());
        assert_eq!(
            chord_notes(&entry.degrees, "F#"),
            expected,
            "進行 {} でハイフンとスペースの結果が食い違う",
            entry.degrees
        );
    }
}

/// ルート音 [A-G] や臨時記号の直後のハイフンだけはコード品質（`C-7` = Cm7）。
/// degree 表記（I / IV / bVII …）は A〜G で終わらないのでこれには当たらない。
#[test]
fn a_hyphen_after_a_root_letter_is_a_chord_quality_not_a_separator() {
    assert_eq!(
        crate::note_progression("C-7").unwrap(),
        vec![vec![60, 63, 67, 70]],
        "C-7 は Cm7 の1コードであって C と 7 の2コードではない"
    );
    // 区切りとして働く位置のハイフンは、ちゃんと2コードに分かれる。
    assert_eq!(crate::note_progression("C-Dm").unwrap().len(), 2);
}

#[test]
fn unparsable_notation_is_rejected_instead_of_being_treated_as_raw_mml() {
    // 生 MML へのフォールバックを通さないので、convert の Err がそのまま伝わる。
    let error = chord_notes("cdefg", "C").unwrap_err();
    assert!(error.contains("コード進行を解釈できません"), "{error}");
    assert!(chord_notes("zzz", "C").is_err());
    // 同じ入力を note_progression へ渡すと、生 MML として鳴ってしまう（keyboard の仕様）。
    assert!(crate::note_progression("Key:C cdefg").is_ok());
}

/// 全カタログ × 全 Key を実際に変換し、進行ごとの成否が Key に依存しないことと、
/// 演奏可能な進行が十分に残ることを確かめる。
#[test]
fn shipped_catalog_converts_for_every_key() {
    let catalog = catalog();
    let mut playable = Vec::new();
    let mut failed = Vec::new();
    for entry in catalog.entries() {
        let results = KEYS
            .iter()
            .map(|key| chord_notes(&entry.degrees, key).map_err(|error| (*key, error)))
            .collect::<Vec<_>>();
        let ok_count = results.iter().filter(|result| result.is_ok()).count();
        assert!(
            ok_count == 0 || ok_count == KEYS.len(),
            "進行 {} の成否が Key によって変わる（成功 {ok_count}/{}）: {:?}",
            entry.degrees,
            KEYS.len(),
            results.iter().find_map(|r| r.as_ref().err()),
        );
        if ok_count == KEYS.len() {
            for result in &results {
                let chords = result.as_ref().expect("checked above");
                assert_eq!(
                    chords.len(),
                    entry.chord_count(),
                    "進行 {} のコード数が degrees と一致しない",
                    entry.degrees
                );
                assert!(
                    chords.iter().all(|chord| !chord.is_empty()),
                    "進行 {} に空の和音がある",
                    entry.degrees
                );
            }
            playable.push(entry.degrees.clone());
        } else {
            failed.push((
                entry.degrees.clone(),
                results[0].as_ref().unwrap_err().1.clone(),
            ));
        }
    }
    assert_eq!(
        playable.len(),
        catalog.len(),
        "変換できない進行がある。失敗一覧: {failed:#?}"
    );
}

#[test]
fn pick_playable_returns_a_convertible_progression() {
    let catalog = catalog();
    let pick = catalog.pick_playable(64).expect("カタログから1つは引ける");
    assert!(KEYS.contains(&pick.key));
    assert!(!pick.chords.is_empty());
    assert_eq!(
        pick.chords,
        chord_notes(&pick.degrees, pick.key).unwrap(),
        "引いた進行は同じ結果を再現できる"
    );
}

#[test]
fn pick_playable_on_empty_catalog_is_none() {
    assert!(ChordProgressionCatalog::default()
        .pick_playable(8)
        .is_none());
}
