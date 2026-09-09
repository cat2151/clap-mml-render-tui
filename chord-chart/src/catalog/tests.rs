use super::*;

fn source_of(progressions: &[&str]) -> ChordProgressionSource {
    let progressions: Vec<String> = progressions
        .iter()
        .map(|text| (*text).to_string())
        .collect();
    ChordProgressionSource::new(Arc::new(move || progressions.clone()))
}

/// 引いたものは**そのまま**返る（解釈できるかは見ない）。
#[test]
fn the_drawn_progression_is_returned_untouched() {
    let source = source_of(&["I-IV-V-I"]);

    assert_eq!(pick_progression(Some(&source)), Ok("I-IV-V-I".into()));
}

/// カタログは外部データなので、この画面が読めないものも混ざり得る。**それも採る**
/// （degrees を解釈しないのだから、良し悪しを判定する道具をこの crate は持たない）。
#[test]
fn a_progression_this_screen_cannot_read_is_picked_all_the_same() {
    let source = source_of(&["zzz"]);

    assert_eq!(pick_progression(Some(&source)), Ok("zzz".into()));
}

/// 引き直しをしないので、カタログの中身は等しく引かれる。
#[test]
fn every_entry_of_the_catalog_can_come_out() {
    let source = source_of(&["zzz", "I-IV-V-I"]);

    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..200 {
        seen.insert(pick_progression(Some(&source)).unwrap());
    }

    assert_eq!(
        seen,
        ["I-IV-V-I".to_string(), "zzz".to_string()]
            .into_iter()
            .collect()
    );
}

/// 未注入と空カタログは、画面から見れば同じ「データが無い」。同じ文言にする。
#[test]
fn a_missing_or_empty_catalog_says_there_is_no_data() {
    assert_eq!(pick_progression(None), Err(NO_CATALOG_MESSAGE.to_string()));
    assert_eq!(
        pick_progression(Some(&source_of(&[]))),
        Err(NO_CATALOG_MESSAGE.to_string())
    );
}

/// 抽選は押した瞬間にだけカタログを引く（画面を開いただけでネットワークを待たない）。
#[test]
fn the_catalog_is_read_only_when_it_is_asked_for() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let source = ChordProgressionSource::new(Arc::new(move || {
        counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        vec!["I-IV-V-I".to_string()]
    }));
    assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 0);

    pick_progression(Some(&source)).unwrap();

    assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
}

/// `Debug` を derive できない型を screen が抱えるので、手書きの実装が残っていること。
#[test]
fn the_source_is_debug_printable_without_showing_the_closure() {
    assert_eq!(
        format!("{:?}", source_of(&["I-IV-V-I"])),
        "ChordProgressionSource(..)"
    );
}
