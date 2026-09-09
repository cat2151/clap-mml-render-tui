use super::*;

/// テスト専用の history ディレクトリへ差し替えて `body` を走らせる。
///
/// `set_local_dir_envs` はプロセス全体の env lock を取るので、
/// このテスト同士および他 crate のテストと直列化される。
fn with_temp_history<T>(name: &str, body: impl FnOnce() -> T) -> T {
    let tmp = std::env::temp_dir().join(format!("cmrt_test_chord_chart_{name}"));
    std::fs::remove_dir_all(&tmp).ok();
    let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
    let result = body();
    std::fs::remove_dir_all(&tmp).ok();
    result
}

/// 保存先へ生のバイト列を直接置く（壊れた JSON を作るため）。
fn write_raw(content: &str) {
    let path = cmrt_history::chord_chart_file_path().expect("保存先が決まること");
    std::fs::create_dir_all(path.parent().expect("親ディレクトリがあること")).unwrap();
    std::fs::write(path, content).unwrap();
}

/// 読めることを期待して読む。
fn loaded() -> Song {
    load_song().expect("保存済みの曲を読めること")
}

/// JSON object のキーを sorted で取り出す。
///
/// 「このキーが無いこと」を名前で書くと、その綴り自体が受け入れ条件の grep に
/// 引っかかる（Stage 2 の罠）。集合の等値比較なら綴りを書かずに済む。
fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap_or_else(|| panic!("object であること: {value}"))
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

#[test]
fn a_saved_song_comes_back_unchanged() {
    with_temp_history("roundtrip", || {
        let mut song = Song::empty();
        song.prefix = "Key=F# BPM96".to_string();
        let a = song.push_section("A", "I-V-VIm-IV");
        let b = song.push_section("B", "IIm-V-I-VIm");
        song.arrangement = vec![a, a, b];

        save_song(&song).expect("保存できること");

        assert_eq!(loaded(), song);
    });
}

/// ファイルが無いのは「空の曲」ではなく **`None`**。
/// 呼び出し側（画面）はこれを見て、初回だけ自動抽選する。
#[test]
fn a_missing_file_is_reported_as_none() {
    with_temp_history("missing", || {
        let path = cmrt_history::chord_chart_file_path().expect("保存先が決まること");
        assert!(!path.exists(), "テスト開始時にファイルが無いこと");
        assert_eq!(load_song(), None);
    });
}

#[test]
fn a_broken_file_is_reported_as_none_without_panicking() {
    with_temp_history("broken", || {
        for broken in ["{", "", "null", "[1,2,3]", "\u{0}not json"] {
            write_raw(broken);
            assert_eq!(
                load_song(),
                None,
                "壊れた内容 {broken:?} で None になること"
            );
        }
    });
}

#[test]
fn an_arrangement_entry_without_a_section_is_dropped() {
    with_temp_history("dangling_arrangement", || {
        write_raw(
            r#"{
              "prefix": "Key=C BPM120",
              "sections": [
                { "id": 1, "name": "A", "degrees": "I-V-VIm-IV" }
              ],
              "arrangement": [1, 99, 1]
            }"#,
        );
        let song = loaded();
        assert_eq!(song.arrangement.len(), 2, "id 99 の行だけ落ちること");
        assert!(song
            .arrangement
            .iter()
            .all(|id| song.section(*id).is_some()));
    });
}

/// prefix は**検証しない**ので、chord2mml が読めない綴りでもそのまま往復する。
/// 弾いてしまうと「打った文字列が黙って消える」画面になる。
#[test]
fn an_unparsable_prefix_survives_the_round_trip_untouched() {
    with_temp_history("free_prefix", || {
        write_raw(
            r#"{
              "prefix": "Key=H BPM:0 好きに書ける",
              "sections": [
                { "id": 1, "name": "A", "degrees": "I-V-VIm-IV" },
                { "id": 2, "name": "B", "degrees": "I-V" }
              ],
              "arrangement": [1]
            }"#,
        );
        let song = loaded();
        assert_eq!(song.prefix, "Key=H BPM:0 好きに書ける");
        assert_eq!(song.sections.len(), 2);
    });
}

/// 削減前の形式（`version` / `title` / `key` / `bpm` / 1 コードの小節数）。
///
/// **移行コードは書かない**（資料 7 章 4）。消したフィールドは serde が黙って無視し、
/// 無かった prefix は既定値へ落ちる。panic せず section が読めれば十分。
#[test]
fn the_old_format_is_read_without_a_panic() {
    with_temp_history("legacy_format", || {
        write_raw(
            r#"{
              "version": 1,
              "title": "曲名",
              "key": "F#",
              "bpm": 96.0,
              "sections": [
                { "id": 1, "name": "A", "degrees": "I-V-VIm-IV", "measures_per_chord": 2 }
              ],
              "arrangement": [1]
            }"#,
        );
        let song = loaded();
        assert_eq!(song.prefix, "Key=C BPM120", "旧 key / bpm は復元しない");
        assert_eq!(song.sections.len(), 1);
        assert_eq!(song.arrangement.len(), 1);
    });
}

#[test]
fn missing_fields_do_not_stop_the_file_from_loading() {
    with_temp_history("missing_fields", || {
        write_raw(r#"{ "sections": [ { "id": 3 } ] }"#);
        let song = loaded();
        assert_eq!(song.prefix, "Key=C BPM120");
        assert_eq!(song.sections.len(), 1);
        // degrees 未指定は空文字。検証しないので、そのまま空の行として残る。
        assert_eq!(song.sections[0].degrees, "");
        assert!(song.arrangement.is_empty());
    });
}

/// section を全部消して保存した曲は `Some(空)`。**`None` と混ぜてはいけない**
/// （混ぜると、全部消してから再起動したときに抽選で section が復活する）。
#[test]
fn a_song_saved_with_no_sections_is_loaded_as_an_empty_song_not_as_a_failure() {
    with_temp_history("saved_empty", || {
        save_song(&Song::empty()).expect("保存できること");
        assert_eq!(load_song(), Some(Song::empty()));
    });
}

#[test]
fn ids_issued_after_a_load_never_collide_with_the_loaded_ones() {
    with_temp_history("reseed", || {
        write_raw(
            r#"{
              "prefix": "Key=C BPM120",
              "sections": [
                { "id": 7, "name": "A", "degrees": "I-V" },
                { "id": 3, "name": "B", "degrees": "I-V" }
              ],
              "arrangement": [7]
            }"#,
        );
        let mut song = loaded();
        // `g`（抽選追加）に相当する操作。
        let added = song.push_section("C", "I-IV");
        assert_eq!(added.get(), 8, "最大 id + 1 から発番されること");
        let ids: Vec<u32> = song.sections.iter().map(|s| s.id.get()).collect();
        assert_eq!(ids, vec![7, 3, 8]);
    });
}

#[test]
fn a_duplicated_id_in_the_file_is_kept_only_once() {
    with_temp_history("duplicate_id", || {
        write_raw(
            r#"{
              "prefix": "Key=C BPM120",
              "sections": [
                { "id": 1, "name": "A", "degrees": "I-V" },
                { "id": 1, "name": "B", "degrees": "I-V-VIm-IV" }
              ],
              "arrangement": [1]
            }"#,
        );
        let song = loaded();
        assert_eq!(song.sections.len(), 1);
        assert_eq!(song.sections[0].name, "A", "先に出たほうを残すこと");
    });
}

#[test]
fn the_saved_json_matches_the_documented_shape() {
    with_temp_history("shape", || {
        let mut song = Song::empty();
        let a = song.push_section("A", "I-V-VIm-IV");
        song.arrangement = vec![a, a];
        save_song(&song).expect("保存できること");

        let path = cmrt_history::chord_chart_file_path().expect("保存先が決まること");
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("chord_chart.json")
        );
        let raw = std::fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        // Key / BPM は 1 本の文字列。旧形式の `key` / `bpm` はもう書き出さない。
        assert_eq!(value["prefix"], "Key=C BPM120");
        // SectionId は newtype なので裸の数値で並ぶ（資料 4.3 の形）。
        assert_eq!(value["arrangement"], serde_json::json!([1, 1]));
        assert_eq!(value["sections"][0]["degrees"], "I-V-VIm-IV");
        // ルートが持つのはこの 3 つだけ。読む側が見ないバージョン番号も、曲名も、
        // もう書き出さない。
        assert_eq!(
            sorted_keys(&value),
            vec!["arrangement", "prefix", "sections"],
            "{value}"
        );
        // section が持つのはこの 3 つだけ。
        assert_eq!(
            sorted_keys(&value["sections"][0]),
            vec!["degrees", "id", "name"],
            "{value}"
        );
    });
}
