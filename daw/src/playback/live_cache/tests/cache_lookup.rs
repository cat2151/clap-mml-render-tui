use crate::playback::live_cache::ready_cache_wav_for_measure;
use crate::tracks::FIRST_PLAYABLE_TRACK;

/// セルを編集すると、その小節のキャッシュ WAV が消えて live 経路は無音になる。
///
/// 判断 1（「まだ出来ていない小節は無音のまま」）が成り立つ前提は、**編集で WAV が
/// 実際に消えること**。ここが崩れると、演奏ループは古い WAV を鳴らし続けてしまい、
/// 「編集したのに前の音が鳴る」という一番たちの悪い形の嘘になる。
/// 実サーバーもユーザーの実キャッシュも要らない（temp の cache dir で完結する）。
#[test]
fn editing_a_cell_removes_its_cache_wav_so_that_measure_falls_silent() {
    let tmp = std::env::temp_dir().join("cmrt_test_live_cache_invalidates_wav");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
        let (mut app, _cache_rx) = crate::input::tests::build_test_app();
        let row = FIRST_PLAYABLE_TRACK;
        app.editor.data[row][1] = "cde".to_string();
        app.sync_cache_states();

        // render 済みのキャッシュがある状態を作る（中身は読まないので実 WAV でなくてよい）。
        let cache_wav = crate::cache::cache_wav_path(app.workspace_kind, row, 1)
            .expect("row 2 / meas 1 has a cache path");
        std::fs::write(&cache_wav, b"cached audio").unwrap();
        assert_eq!(
            ready_cache_wav_for_measure(app.workspace_kind, 0, row),
            Some(cache_wav.clone()),
            "the loop should see the cache while it exists"
        );

        app.invalidate_cell(row, 1);

        assert!(!cache_wav.exists(), "editing the cell removes the wav");
        assert_eq!(
            ready_cache_wav_for_measure(app.workspace_kind, 0, row),
            None,
            "with no wav the loop sends nothing, so the measure is silent"
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

/// **今の（壊れている）振る舞いを記録した緑。** 直したらここは赤くなる。
///
/// 演奏ループがキャッシュを引く唯一の入口 [`ready_cache_wav_for_measure`] は
/// **「ファイルが在るか」しか見ない。** hash も `CacheState` も WAV の長さも
/// editor の中身も見ない。だから **今日の project に 1 文字も無い行でも、
/// 前日のファイルがその名前を占めていれば鳴ってしまう**
/// （キャッシュのファイル名は `track{行}_meas{小節}.wav` で、日付も hash も入らない）。
///
/// これが「前日の音が混ざる」の芯。`measure_live_cues` まで通してあるので、
/// 「引ける」だけでなく **その行が `cues` に載る（＝実際に note on が送られる）**
/// ところまで固定している。
///
/// 直す（例: `CacheState::Ready` と hash 一致を通す）と、この行は `silent_rows`
/// 側へ移って **この 2 本の assert が赤くなる**。それが正しい向き。
#[test]
fn a_row_with_no_content_still_sounds_when_a_stale_cache_wav_remains() {
    let tmp = std::env::temp_dir().join("cmrt_test_live_cache_stale_wav_sounds");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
        let (app, _cache_rx) = crate::input::tests::build_test_app();
        let stale_row = FIRST_PLAYABLE_TRACK;
        let empty_row = FIRST_PLAYABLE_TRACK + 1;

        // 今日の project は空。どのセルにも 1 文字も入れない。
        app.sync_cache_states();
        assert!(
            app.editor.data[stale_row][1].is_empty(),
            "the cell must stay empty; that is the whole point of this test"
        );
        assert!(
            app.cache.lock().unwrap()[stale_row][1].state == crate::CacheState::Empty,
            "an empty cell is Empty, so kick_all_pending() never re-renders it"
        );

        // 前日の WAV だけが残っている状況（中身は読まれないので実 WAV でなくてよい）。
        let stale_wav = crate::cache::cache_wav_path(app.workspace_kind, stale_row, 1)
            .expect("row 2 / meas 1 has a cache path");
        std::fs::write(&stale_wav, b"yesterday's audio").unwrap();

        assert_eq!(
            ready_cache_wav_for_measure(app.workspace_kind, 0, stale_row),
            Some(stale_wav.clone()),
            "the loop only checks that the file exists, so the stale wav is picked up"
        );

        let cues = crate::playback::live_cache::measure_live_cues(app.editor.tracks, |row| {
            ready_cache_wav_for_measure(app.workspace_kind, 0, row)
        });

        assert_eq!(
            cues.cues.iter().map(|cue| cue.row).collect::<Vec<_>>(),
            vec![stale_row],
            "the empty row is sent to the live path purely because a stale wav exists"
        );
        assert_eq!(
            cues.silent_rows,
            vec![empty_row],
            "only the row without a leftover file falls silent"
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}
