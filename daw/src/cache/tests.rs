use std::path::Path;

use super::workspace_cache_dir;
use crate::WorkspaceKind;

#[test]
fn workspace_cache_directory_keeps_persistent_path_and_nests_daily() {
    let plugin_namespace = Path::new("daw_cache").join("Surge XT");

    assert_eq!(
        workspace_cache_dir(&plugin_namespace, WorkspaceKind::Persistent),
        plugin_namespace
    );
    assert_eq!(
        workspace_cache_dir(&plugin_namespace, WorkspaceKind::Daily),
        plugin_namespace.join("daily")
    );
}

/// chord 行の中身はコード進行なので、レンダリングのジョブを作らない。
///
/// ここで止めないと chord2mml の文字列がそのまま MML パーサへ流れ、
/// 無音のセルを毎回レンダリングし続けることになる。
#[test]
fn the_chord_row_never_produces_a_cache_job() {
    let (mut app, cache_rx) = crate::input::tests::build_test_app();
    app.editor.data[crate::CHORD_TRACK][1] = "I-IV-V-I".to_string();
    app.editor.data[crate::FIRST_PLAYABLE_TRACK][1] = "cde".to_string();

    app.sync_cache_states();
    app.kick_cache(crate::CHORD_TRACK, 1);
    app.kick_cache(crate::FIRST_PLAYABLE_TRACK, 1);

    let jobs: Vec<usize> = std::iter::from_fn(|| cache_rx.try_recv().ok())
        .map(|job| job.track)
        .collect();

    assert_eq!(jobs, vec![crate::FIRST_PLAYABLE_TRACK]);
    let cache = app.cache.lock().unwrap();
    assert!(cache[crate::CHORD_TRACK][1].state == crate::CacheState::Empty);
}

/// 手書きセルが空でも、chord 行から生成されるセルはレンダリング対象になる。
///
/// 生のセル文字列だけで空判定していると、生成されたセルが丸ごと無視されて音が出ない。
#[test]
fn a_cell_generated_from_the_chord_row_is_queued_even_though_the_cell_itself_is_empty() {
    let (mut app, cache_rx) = crate::input::tests::build_test_app();
    app.editor.data[crate::CHORD_TRACK][1] = "I-IV".to_string();
    app.editor.data[crate::FIRST_PLAYABLE_TRACK][0] =
        r#"{"generate from chord track": "close"}"#.to_string();

    app.sync_cache_states();
    assert!(
        app.cache.lock().unwrap()[crate::FIRST_PLAYABLE_TRACK][1].state
            == crate::CacheState::Pending
    );

    app.kick_cache(crate::FIRST_PLAYABLE_TRACK, 1);

    let jobs: Vec<crate::CacheJob> = std::iter::from_fn(|| cache_rx.try_recv().ok()).collect();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].track, crate::FIRST_PLAYABLE_TRACK);
    // chord2mml.exe "close | I-IV |" の出力そのまま。
    assert!(
        jobs[0].mml.contains("v11/*|*/'c2eg''f2a<c'/*|*/"),
        "unexpected mml: {}",
        jobs[0].mml
    );
}

/// chord 行を編集したら、そこから生成されているセルのキャッシュが捨てられる。
///
/// 手書きのセルは chord 行に依存しないので巻き込まない。
#[test]
fn editing_the_chord_row_invalidates_only_the_cells_generated_from_it() {
    let (mut app, _cache_rx) = crate::input::tests::build_test_app();
    let generated = crate::FIRST_PLAYABLE_TRACK;
    let handwritten = crate::FIRST_PLAYABLE_TRACK + 1;
    app.editor.data[crate::CHORD_TRACK][1] = "I-IV".to_string();
    app.editor.data[generated][0] = r#"{"generate from chord track": ""}"#.to_string();
    app.editor.data[handwritten][0] = r#"{"generate from chord track": ""}"#.to_string();
    app.editor.data[handwritten][1] = "cde".to_string();
    app.sync_cache_states();

    let affected = app.invalidate_dependent_cells(crate::CHORD_TRACK, 1);

    assert_eq!(affected, vec![(generated, 1)]);
}

/// 「Resume の日」に通る復元経路 [`super::DawApp::restore_cache_from_metadata`] を
/// 実際に通すための道具立て。
///
/// - `set_local_dir_envs` で cache dir を temp へ逃がす（実 `%LOCALAPPDATA%` を汚さない）
/// - WAV は hound で実際に書く。`read_wav_cache_info` が RIFF を読むので、
///   ダミーのバイト列では経路が変わってしまう
mod restore_from_metadata {
    use cmrt_history::{daw_cache_mml_hash, DawCachedMeasure};

    use crate::{CacheState, FIRST_PLAYABLE_TRACK};

    /// 44.1kHz f32 ステレオの無音 WAV を書く。`interleaved_samples` は L/R 込みの総サンプル数。
    fn write_silent_stereo_wav(path: &std::path::Path, interleaved_samples: usize) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for _ in 0..interleaved_samples {
            writer.write_sample(0.0f32).unwrap();
        }
        writer.finalize().unwrap();
    }

    /// テンポだけを変えると `mml_hash` が変わるので、前日の WAV は Ready にならない。
    ///
    /// `build_cell_mml` は track0（`{"beat":"4/4"}t113` など）を各セルの先頭に前置するので、
    /// **テンポは hash に効く**。つまり Resume 経路には「前日のテンポの WAV を今日の
    /// セルへ貼る」穴は無い。**ここは白。**
    ///
    /// （2026-09-04 の事象で前日の音が鳴ったのは rollover 経路で、そちらは
    /// `restore_cache_from_metadata` を 1 度も通らない。混同しないこと）
    #[test]
    fn a_tempo_only_change_keeps_yesterdays_wav_out_of_the_restored_cache() {
        let tmp = std::env::temp_dir().join("cmrt_test_daw_restore_tempo_change");
        std::fs::remove_dir_all(&tmp).ok();

        {
            let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
            let (mut app, _cache_rx) = crate::input::tests::build_test_app();
            let row = FIRST_PLAYABLE_TRACK;

            // 前日（t113）に焼いたときの hash。
            app.editor.data[0][0] = r#"{"beat": "4/4"}t113"#.to_string();
            app.editor.data[row][1] = "cde".to_string();
            let yesterday_hash = daw_cache_mml_hash(&app.build_cell_mml(row, 1));
            let path = super::super::cache_wav_path(app.workspace_kind, row, 1).unwrap();
            write_silent_stereo_wav(&path, app.measure_duration_samples());

            // 今日はテンポだけ変えた（t113 -> t120）。ほかは 1 文字も変えていない。
            app.editor.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
            let today_hash = daw_cache_mml_hash(&app.build_cell_mml(row, 1));
            assert_ne!(
                yesterday_hash, today_hash,
                "the tempo lives in track0 and is prefixed onto every cell, so it must move the hash"
            );

            app.restore_cache_from_metadata(&[DawCachedMeasure {
                track: row,
                measure: 1,
                mml_hash: yesterday_hash,
                legacy_mml: None,
            }]);

            let cell = &app.cache.lock().unwrap()[row][1];
            assert!(
                cell.state != CacheState::Ready,
                "a hash mismatch must not be restored as Ready"
            );
            assert_eq!(cell.rendered_mml_hash, None);
            assert!(
                path.is_file(),
                "the restore path does not delete the wav either; only the cache state is withheld"
            );
        }

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// **今の（穴のある）振る舞いを記録した緑。** 直したらここは赤くなる。
    ///
    /// `restore_cache_from_metadata` は WAV の `interleaved_sample_count` を
    /// **今日の小節長と 1 度も突き合わせない**。sample_rate と channels しか見ないので、
    /// **長さが今日の小節長と違う WAV でも Ready になり**、しかも
    /// `rendered_measure_samples` には **実測ではなく今日の値**が貼られる。
    ///
    /// hash が一致していれば長さも一致するはず、という前提に寄りかかった造り。
    /// hash が一致したまま長さだけ壊れる経路（書き込み中のファイル、外から差し替えた
    /// ファイル、renderer の release 長の変更）では、この札が嘘になる。
    ///
    /// 直す（長さを突き合わせて Pending へ落とす）と `state == Ready` の assert が
    /// 赤くなる。それが正しい向き。
    #[test]
    fn a_wav_whose_length_does_not_match_todays_measure_is_still_restored_as_ready() {
        let tmp = std::env::temp_dir().join("cmrt_test_daw_restore_length_mismatch");
        std::fs::remove_dir_all(&tmp).ok();

        {
            let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
            let (mut app, _cache_rx) = crate::input::tests::build_test_app();
            let row = FIRST_PLAYABLE_TRACK;

            app.editor.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
            app.editor.data[row][1] = "cde".to_string();
            let hash = daw_cache_mml_hash(&app.build_cell_mml(row, 1));

            // hash は今日のまま・長さだけ別世代（BPM 113 相当に伸びている）。
            let todays_samples = app.measure_duration_samples();
            let stale_length = todays_samples + 12_346;
            let path = super::super::cache_wav_path(app.workspace_kind, row, 1).unwrap();
            write_silent_stereo_wav(&path, stale_length);
            assert_eq!(
                cmrt_tui_core::wav_io::read_wav_cache_info(&path)
                    .unwrap()
                    .interleaved_sample_count,
                stale_length,
                "the file on disk really is a different length from today's measure"
            );

            app.restore_cache_from_metadata(&[DawCachedMeasure {
                track: row,
                measure: 1,
                mml_hash: hash,
                legacy_mml: None,
            }]);

            let cell = &app.cache.lock().unwrap()[row][1];
            assert!(
                cell.state == CacheState::Ready,
                "length is never checked, so the mismatched wav is accepted"
            );
            assert_eq!(
                cell.rendered_measure_samples,
                Some(todays_samples),
                "and today's measure length is stamped on regardless of what was actually read"
            );
            assert_eq!(
                cell.samples.as_ref().map(|samples| samples.len()),
                Some(stale_length),
                "the samples actually held are the stale ones, so the stamp above is a lie"
            );
        }

        std::fs::remove_dir_all(&tmp).ok();
    }
}
