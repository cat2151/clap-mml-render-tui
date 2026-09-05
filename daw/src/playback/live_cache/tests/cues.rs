use cmrt_core::cache_wav::SLOT_COUNT;

use super::wav;
use crate::live_instance::MAX_LIVE_TRACKS;
use crate::playback::live_cache::cues::{
    measure_live_cues, measure_slot, note_on_events, LiveCacheCue,
};
use crate::tracks::FIRST_PLAYABLE_TRACK;

/// timeline id は 0 以外なら何でもよい（サーバーは 0 を「timeline 無し」として弾く）。
const TIMELINE_ID: u64 = 7;

/// 全行にキャッシュがある小節は、行 2 から順に instance 0 から埋まる。
#[test]
fn every_row_with_a_cache_gets_the_instance_that_matches_its_grid_row() {
    let cues = measure_live_cues(5, |row| Some(wav(row)));

    assert_eq!(
        cues.cues,
        vec![
            LiveCacheCue {
                row: 2,
                instance: 0,
                wav: wav(2)
            },
            LiveCacheCue {
                row: 3,
                instance: 1,
                wav: wav(3)
            },
            LiveCacheCue {
                row: 4,
                instance: 2,
                wav: wav(4)
            },
        ]
    );
    assert!(cues.silent_rows.is_empty());
    assert!(cues.rows_over_instance_limit.is_empty());
}

/// Tempo 行（0）と chord 行（1）は音を鳴らさないので、送信対象にも無音扱いにも入らない。
#[test]
fn the_tempo_and_chord_rows_are_not_part_of_the_measure_at_all() {
    let cues = measure_live_cues(4, |row| Some(wav(row)));

    let touched: Vec<usize> = cues
        .cues
        .iter()
        .map(|cue| cue.row)
        .chain(cues.silent_rows.iter().copied())
        .chain(cues.rows_over_instance_limit.iter().copied())
        .collect();
    assert_eq!(touched, vec![2, 3]);
    assert_eq!(FIRST_PLAYABLE_TRACK, 2);
}

/// キャッシュがまだ無い行には何も送らない（承認済みの設計判断 1: そこは無音のまま）。
#[test]
fn a_row_without_a_cached_wav_is_left_silent_instead_of_reusing_the_previous_measure() {
    let cues = measure_live_cues(5, |row| (row != 3).then(|| wav(row)));

    assert_eq!(
        cues.cues.iter().map(|cue| cue.row).collect::<Vec<_>>(),
        vec![2, 4]
    );
    assert_eq!(cues.silent_rows, vec![3]);
    // 送らない行の instance は空いたままにする（詰めて別の行を鳴らしたりしない）。
    assert_eq!(
        cues.cues.iter().map(|cue| cue.instance).collect::<Vec<_>>(),
        vec![0, 2]
    );
}

/// instance 数はサーバー起動時にしか決まらないので、溢れた行は鳴らさず内訳へ落とす。
#[test]
fn rows_beyond_the_server_instance_limit_are_reported_instead_of_played() {
    let last_playable_row = FIRST_PLAYABLE_TRACK + MAX_LIVE_TRACKS - 1;
    let cues = measure_live_cues(last_playable_row + 3, |row| Some(wav(row)));

    assert_eq!(cues.cues.len(), MAX_LIVE_TRACKS);
    assert_eq!(
        cues.cues.last().map(|cue| (cue.row, cue.instance)),
        Some((last_playable_row, (MAX_LIVE_TRACKS - 1) as u8))
    );
    assert_eq!(
        cues.rows_over_instance_limit,
        vec![last_playable_row + 1, last_playable_row + 2]
    );
    assert!(cues.silent_rows.is_empty());
}

/// 演奏 track が 1 つも無いグリッドでは送信も無音行も生まれない。
#[test]
fn a_grid_without_playable_rows_produces_nothing_to_send() {
    let cues = measure_live_cues(FIRST_PLAYABLE_TRACK, |row| Some(wav(row)));

    assert!(cues.cues.is_empty());
    assert!(cues.silent_rows.is_empty());
    assert!(cues.rows_over_instance_limit.is_empty());
}

/// **隣り合う小節は必ず別スロット**になる。
///
/// これが先読みの土台。同じスロットへ落ちると、次の小節を載せた瞬間に
/// いま鳴らそうとしている小節を上書きしてしまう。
#[test]
fn neighbouring_measures_never_share_a_slot() {
    let slots: Vec<usize> = (0..2 * SLOT_COUNT).map(measure_slot).collect();

    assert_eq!(
        slots,
        (0..2 * SLOT_COUNT)
            .map(|i| i % SLOT_COUNT)
            .collect::<Vec<_>>()
    );
    for pair in slots.windows(2) {
        assert_ne!(pair[0], pair[1], "隣の小節と同じスロットになっている");
    }
    // スロットを増やしてもこの性質は保たれる（`% SLOT_COUNT` なので）。
    const { assert!(SLOT_COUNT >= 2, "先読みには最低 2 スロット要る") };
}

/// 全 track の note on が、同じ発音位置を指す 1 バッチになる。
///
/// ここが 1 件ずつのコマンドへ戻ると、`prepare` の応答待ちが note on のあいだに挟まって
/// 小節の頭が track ごとにずれる（対策前の実測で 8 track / 250ms）。
#[test]
fn every_track_gets_its_note_on_in_one_batch_at_the_same_position() {
    let cues = measure_live_cues(5, |row| (row != 3).then(|| wav(row)));

    let events = note_on_events(&cues.cues, measure_slot(0), TIMELINE_ID, 5.0);
    assert_eq!(
        events
            .iter()
            .map(|event| event.instance_id)
            .collect::<Vec<_>>(),
        vec![0, 2],
        "キャッシュのある行だけが、その行の instance で鳴る"
    );
    assert!(
        events.iter().all(|event| event.timeline_seconds == 5.0),
        "小節の頭に揃えるので発音位置は全部同じ: {events:?}"
    );
    assert!(
        events.iter().all(|event| event.timeline_id == TIMELINE_ID),
        "1 バッチは 1 つの timeline へ載る（サーバーは混在を弾く）: {events:?}"
    );
}

/// **発音位置は送った時刻ではなく、指定した絶対秒**。
///
/// ここが「届いた瞬間のオーディオブロック」へ戻ると、小節ごとに −42.7〜+21.3ms
/// ずれる（対策前の実測）。
#[test]
fn the_note_on_sounds_at_the_absolute_second_it_was_given() {
    let cues = measure_live_cues(3, |row| Some(wav(row)));

    let at = |seconds: f64| note_on_events(&cues.cues, 0, TIMELINE_ID, seconds)[0].timeline_seconds;
    assert_eq!(at(0.0), 0.0);
    assert_eq!(at(2.133_333_333_333_333_3), 2.133_333_333_333_333_3);
}

/// note number が、その小節を載せたスロットを指す。
///
/// cache-player は `note % SLOT_COUNT` でスロットを選ぶので、ここがずれると
/// **1 つ前（か 1 つ後）の小節が鳴る**。しかも「鳴ってはいる」ので気づきにくい。
#[test]
fn the_note_number_points_at_the_slot_that_holds_this_measure() {
    let cues = measure_live_cues(3, |row| Some(wav(row)));

    let note_of = |measure_index: usize| {
        note_on_events(&cues.cues, measure_slot(measure_index), TIMELINE_ID, 0.0)[0].message[1]
    };
    assert_eq!(note_of(0), 60, "小節 1 はスロット 0 = note 60");
    assert_eq!(note_of(1), 61, "小節 2 はスロット 1 = note 61");
    assert_eq!(
        note_of(SLOT_COUNT),
        60,
        "スロットを 1 周したらスロット 0 = note 60 へ戻る"
    );
    // cache-player 側の `slot_for_note` と同じ剰余になっていること。
    for measure_index in 0..2 * SLOT_COUNT {
        assert_eq!(
            usize::from(note_of(measure_index)) % SLOT_COUNT,
            measure_slot(measure_index)
        );
    }
}

/// 鳴らすものが無い小節では 1 コマンドも送らない。
///
/// サーバーは空の MIDI バッチを `InvalidPayload` で弾く（1..=128 件しか受けない）。
/// 「キャッシュがまだ無いので無音」は正常な状態（判断 1）なので、エラーにしてはいけない。
#[test]
fn a_measure_with_nothing_to_play_produces_no_midi_batch_at_all() {
    let cues = measure_live_cues(5, |_| None);

    assert!(note_on_events(&cues.cues, measure_slot(0), TIMELINE_ID, 0.0).is_empty());
}
