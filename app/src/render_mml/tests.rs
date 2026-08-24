use super::*;

use cmrt_runtime::{
    CatalogPlugin, DEXED_PLUGIN_ID, FLOE_PLUGIN_ID, SURGE_XT_PLUGIN_ID, VAPORIZER2_PLUGIN_ID,
};

fn plugin(name: &str, plugin_id: &str) -> CatalogPlugin {
    CatalogPlugin {
        name: name.to_string(),
        plugin_path: format!("/clap/{name}.clap"),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        resolved_patches: None,
        source_notices: Vec::new(),
    }
}

fn mixed_catalog() -> PatchPlugins {
    PatchPlugins::from_catalog(vec![
        plugin("Dexed", DEXED_PLUGIN_ID),
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        plugin("Vaporizer2", VAPORIZER2_PLUGIN_ID),
        plugin("Floe", FLOE_PLUGIN_ID),
    ])
}

#[test]
fn a_patch_is_embedded_in_the_mml_head_json() {
    assert_eq!(
        mml_with_patch(Some("AR Accent Arp.vvp"), "t120o4l4c"),
        r#"{"Surge XT patch":"AR Accent Arp.vvp"}t120o4l4c"#
    );
}

/// 音色を指定しなければ MML はそのまま。先頭 JSON を足すと「無指定の行」ではなくなる。
#[test]
fn no_patch_leaves_the_mml_untouched() {
    assert_eq!(mml_with_patch(None, "cde"), "cde");
}

/// display 文字列に `"` や `\` が入っても JSON として壊れないこと。
#[test]
fn a_patch_with_json_metacharacters_is_escaped() {
    let line = mml_with_patch(Some(r#"My"Bank\PD Emily.vvp"#), "c");
    let json_end = line.find('}').unwrap() + 1;
    let value: serde_json::Value = serde_json::from_str(&line[..json_end]).unwrap();
    assert_eq!(
        value.get("Surge XT patch").and_then(|v| v.as_str()),
        Some(r#"My"Bank\PD Emily.vvp"#)
    );
}

/// **オフライン経路の本丸の番人。** `.vvp` が Surge の添字へ落ちると、
/// Surge のインスタンスへ Vaporizer2 の state が渡って落ちる（Stage 3 の実測）。
#[test]
fn each_patch_form_reports_its_own_plugin() {
    let catalog = mixed_catalog();
    assert_eq!(
        plugin_name_for(&catalog, Some("AR Accent Arp.vvp")),
        "Vaporizer2"
    );
    assert_eq!(
        plugin_name_for(&catalog, Some("Pads/Pad 1.fxp")),
        "Surge XT"
    );
    assert_eq!(
        plugin_name_for(&catalog, Some("Dexed_01.syx/00 Say Again.")),
        "Dexed"
    );
    assert_eq!(
        plugin_name_for(
            &catalog,
            Some("Celtic Harp/Realistic Celtic Harp.floe-preset")
        ),
        "Floe"
    );
}

/// 音色無指定は必ず既定プラグイン（先頭）。patch 文字列の形で引くと、空文字列が
/// 「cartridge でも `.vvp` でもない」と判定されて Surge 側へ飛ぶ。
#[test]
fn an_unspecified_patch_reports_the_default_plugin() {
    assert_eq!(plugin_name_for(&mixed_catalog(), None), "Dexed");
}

#[test]
fn an_empty_catalog_does_not_panic() {
    let empty = PatchPlugins::from_catalog(Vec::new());
    assert_eq!(plugin_name_for(&empty, None), "(カタログが空)");
}

/// 実測（`SY Analog Taste 001.vvp` / Mono）そのものの数字で mono と読めること。
#[test]
fn a_chord_that_stays_as_loud_as_one_note_reads_as_mono() {
    let measure = PolyCheck::of(0.084866, 0.084866, &[0.084875, 0.084870, 0.084861]);
    assert!((measure.energy_gain - 1.0).abs() < 0.001);
    assert!(
        measure.verdict().starts_with("mono"),
        "{}",
        measure.verdict()
    );
}

/// 実測（`AT Ambience 1.vvp` / Poly16）そのものの数字で poly と読めること。
/// **この音色は同じ MML で波形が毎回変わる**が、音量は変わらないので判定できる。
#[test]
fn a_chord_that_is_louder_than_one_note_reads_as_poly() {
    let measure = PolyCheck::of(0.080775, 0.082642, &[0.048101, 0.052274, 0.047803]);
    assert!(measure.energy_gain > 1.6, "{}", measure.energy_gain);
    assert!(
        measure.verdict().starts_with("poly"),
        "{}",
        measure.verdict()
    );
}

/// 閾値の間は**どちらとも言わない**。黙って poly へ倒すと mono が和音行へ出る。
#[test]
fn an_energy_gain_between_the_thresholds_is_not_judged() {
    let measure = PolyCheck::of(1.2, 1.2, &[1.0, 1.0, 1.0]);
    assert!(
        measure.verdict().starts_with("unclear"),
        "{}",
        measure.verdict()
    );
}

/// 同じ MML で音量が動く音色は、和音と単音の比も同じだけ動く。判定しない。
#[test]
fn a_render_whose_loudness_changes_every_time_is_not_judged() {
    let measure = PolyCheck::of(1.0, 2.0, &[0.3, 0.3, 0.3]);
    assert!(measure.rms_jitter > 0.5);
    assert!(
        measure.verdict().starts_with("unknown"),
        "{}",
        measure.verdict()
    );
}

#[test]
fn a_silent_render_is_not_judged() {
    assert!(PolyCheck::of(0.0, 0.0, &[0.0, 0.0, 0.0])
        .verdict()
        .starts_with("unknown"));
    assert!(PolyCheck::of(0.5, 0.5, &[0.0, 0.0, 0.0])
        .verdict()
        .starts_with("unknown"));
}

#[test]
fn the_default_mml_sounds_one_note() {
    // 判定の土台になる既定 MML。テンポ・音量・音長を書いておかないと config 依存になる。
    let notes = cmrt_chord::mml_note_progression(DEFAULT_MML).unwrap();
    assert_eq!(notes, vec![vec![60]]);
}

/// poly-check の単音は、和音の構成音と 1 対 1 で対応していること。
///
/// **ここがずれると判定が黙って壊れる。** 和音と単音でオクターブや音長が違えば、
/// mono の音色でも「単音と一致しない」＝ poly と報告してしまう。
#[test]
fn the_poly_check_notes_are_exactly_the_notes_of_the_chord() {
    let chord = cmrt_chord::mml_note_progression(POLY_CHECK_CHORD_MML).unwrap();
    assert_eq!(chord.len(), 1, "和音は 1 回の同時発音であること");
    assert_eq!(chord[0].len(), 3, "3 音が同時に鳴ること");

    let singles: Vec<u8> = POLY_CHECK_NOTES
        .iter()
        .map(|(_, mml)| {
            let notes = cmrt_chord::mml_note_progression(mml).unwrap();
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].len(), 1, "単音側は 1 音だけ鳴らすこと");
            notes[0][0]
        })
        .collect();

    let mut sorted_chord = chord[0].clone();
    sorted_chord.sort_unstable();
    let mut sorted_singles = singles.clone();
    sorted_singles.sort_unstable();
    assert_eq!(sorted_singles, sorted_chord);
}

/// 和音と単音の**鳴っている長さ**も揃っていること。
/// 長さが違うと、差の大半が「片方だけ鳴っている尻尾」になって比が意味を失う。
#[test]
fn the_chord_and_the_single_notes_last_the_same_time() {
    let chord = cmrt_chord::timed_performance(POLY_CHECK_CHORD_MML).unwrap();
    for (name, mml) in POLY_CHECK_NOTES {
        let note = cmrt_chord::timed_performance(mml).unwrap();
        assert!(
            (note.duration_seconds - chord.duration_seconds).abs() < 1e-9,
            "{name} の長さが和音と違う: {} vs {}",
            note.duration_seconds,
            chord.duration_seconds
        );
    }
}

#[test]
fn no_patch_option_still_renders_one_line() {
    let request = RenderMmlRequest::default();
    assert_eq!(
        selection::requested_patches(&Config::default(), &mixed_catalog(), &request).unwrap(),
        vec![None]
    );
}

#[test]
fn every_patch_option_becomes_its_own_render() {
    let request = RenderMmlRequest {
        patches: vec!["a.vvp".to_string(), "b.vvp".to_string()],
        ..RenderMmlRequest::default()
    };
    assert_eq!(
        selection::requested_patches(&Config::default(), &mixed_catalog(), &request).unwrap(),
        vec![Some("a.vvp".to_string()), Some("b.vvp".to_string())]
    );
}

#[test]
fn unknown_or_empty_plugin_selection_is_an_error() {
    let unknown = RenderMmlRequest {
        plugin: Some("Unknown".to_string()),
        ..RenderMmlRequest::default()
    };
    assert!(selection::requested_patches(&Config::default(), &mixed_catalog(), &unknown).is_err());

    let empty = RenderMmlRequest {
        plugin: Some("Floe".to_string()),
        ..RenderMmlRequest::default()
    };
    let error = selection::requested_patches(&Config::default(), &mixed_catalog(), &empty)
        .unwrap_err()
        .to_string();
    assert!(error.contains("0 件"), "{error}");
}

#[test]
fn verification_default_mml_spans_multiple_octaves() {
    let notes = cmrt_chord::mml_note_progression(VERIFY_DEFAULT_MML).unwrap();
    let pitches = notes.into_iter().flatten().collect::<Vec<_>>();

    assert!(pitches.iter().min().unwrap() < &48);
    assert!(pitches.iter().max().unwrap() > &72);
}

/// `--out-dir` を渡さなければ WAV は 1 バイトも書かない（環境変数も無いとき）。
#[test]
fn without_an_out_dir_no_wav_is_written() {
    let request = RenderMmlRequest::default();
    if std::env::var_os(WAV_OUT_DIR_ENV).is_some() {
        return;
    }
    assert!(resolve_out_dir(&request).unwrap().is_none());
}

#[test]
fn an_out_dir_is_created_if_it_does_not_exist() {
    let dir = std::env::temp_dir().join(format!("cmrt-render-mml-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let request = RenderMmlRequest {
        out_dir: Some(dir.clone()),
        ..RenderMmlRequest::default()
    };
    assert_eq!(resolve_out_dir(&request).unwrap(), Some(dir.clone()));
    assert!(dir.is_dir());
    let _ = std::fs::remove_dir_all(&dir);
}

/// 先頭 JSON へ埋めた音色が、レンダリング側の読み出しでそのまま戻ること。
///
/// **アポストロフィ入りの実プリセット名**（`AT I'll House Your Grains.vvp` /
/// `FX The Wolves's Cries.vvp`）が要注意。MML の和音記法が `'...'` なので、
/// JSON を通さず素の文字列連結にすると MML 側が壊れる。
#[test]
fn a_patch_name_round_trips_through_the_mml_head_json() {
    for patch in [
        "AR Accent Arp.vvp",
        "AT I'll House Your Grains.vvp",
        "FX The Wolves's Cries.vvp",
        "patches_factory/Pads/Pad 1.fxp",
        "Dexed_01.syx/00 Say Again.",
        "Celtic Harp/Realistic Celtic Harp.floe-preset",
    ] {
        let line = mml_with_patch(Some(patch), POLY_CHECK_CHORD_MML);
        assert_eq!(
            cmrt_core::embedded_patch_ref(&line).as_deref(),
            Some(patch),
            "{patch}"
        );
    }
}
