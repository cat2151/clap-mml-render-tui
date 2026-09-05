//! **演奏ループがサーバーのクロックより先行しても、正しい小節が鳴るか。**
//!
//! `docs/adr/0012-live-clock-drift-is-absorbed-not-eliminated.md` の症状 B が起きた仕組みを、実サーバー無しで
//! そのまま再現する。あの不具合は「予約した位置がずれた」のではなく、
//! **予約どおりの位置で鳴らしたときに、スロットの中身が別の小節へ差し替わっていた**もの。
//!
//! ## 何が先行を生むのか
//! サーバーは state load のあいだ render を回さない。演奏ループは `Instant::now()` から
//! 「クロックは実時間どおり 48kHz で進む」と外挿するので、止まっていたぶんだけ先行する
//! （実測 2.7 秒＝BPM113 で 1.3 小節）。**この前提は今も残っている**（案 B / 案 C は見送り。
//! `docs/adr/0012-live-clock-drift-is-absorbed-not-eliminated.md`）ので、先行しても音が壊れないことを
//! ここで担保する。
//!
//! ## なぜスロット数が効くのか
//! 先読みは 1 小節先まで（`live_cache.rs` の `run`）。小節 `N` はスロット
//! `N % SLOT_COUNT` へ載る。先行が `D` 小節あるとき、スロット `s` へ書く瞬間に
//! そこに居るのは `D` 小節前に書いた小節で、その note on はまだ発火していないことがある。
//! 踏み潰しが起きるのは **`D >= SLOT_COUNT`** のときなので、**吸収できる先行は
//! `SLOT_COUNT - 1` 小節**ぶん。

use cmrt_core::cache_wav::SLOT_COUNT;

use crate::playback::live_cache::cues::measure_slot;
use crate::playback::measure_math::following_measure_index;

/// 1 回の note on で「鳴らしたかった小節」と「そのときスロットに入っていた小節」。
#[derive(Debug, Eq, PartialEq)]
struct Sounded {
    intended: usize,
    actual: Option<usize>,
}

/// 演奏ループを、サーバーのクロックが `drift_measures` 小節ぶん遅れている前提で回す。
///
/// 実装の順序は `LiveCachePlayLoop::run` と同じ:
///
/// 1. 演奏開始の 1 小節目だけ、境界でその小節を載せる
/// 2. その小節を鳴らしている最中に、次の小節を**別スロットへ**載せる
/// 3. note on はサーバーのクロックが予約位置へ着いたときに発火する（＝`drift` 小節あと）
///
/// 返るのは発火した note on の一覧。`actual != intended` があれば「違う小節が鳴った」。
fn play(loop_measures: usize, steps: usize, drift_measures: usize) -> Vec<Sounded> {
    let mut slots: Vec<Option<usize>> = vec![None; SLOT_COUNT];
    let mut sounded = Vec::new();
    // 演奏ループが「いま鳴らしている」ことにしている小節（step ごとに 1 つ進む）。
    let mut measure_of_step = Vec::with_capacity(steps + 1);
    let mut measure = 0usize;
    for _ in 0..=steps {
        measure_of_step.push(measure);
        measure = following_measure_index(measure, loop_measures, None);
    }

    for step in 0..steps {
        if step == 0 {
            // 1 小節目だけは境界で載せる（先読みが無いので）。
            let first = measure_of_step[0];
            slots[measure_slot(first)] = Some(first);
        }
        // サーバーのクロックはここで `step - drift` 小節目の予約位置へ着く。
        if let Some(fired) = step.checked_sub(drift_measures) {
            let intended = measure_of_step[fired];
            sounded.push(Sounded {
                intended,
                actual: slots[measure_slot(intended)],
            });
        }
        // 先読み。**鳴っている最中に、次の小節を別スロットへ載せる。**
        let next = measure_of_step[step + 1];
        slots[measure_slot(next)] = Some(next);
    }
    sounded
}

fn wrong_measures(sounded: &[Sounded]) -> Vec<&Sounded> {
    sounded
        .iter()
        .filter(|s| s.actual != Some(s.intended))
        .collect()
}

/// 先行をどこまで試すか。ここまで無事なら「衝突しない」とみなす。
const PROBE_MAX_DRIFT: usize = 16;

/// ある ループ長で、何小節ぶんの先行まで正しい小節が鳴るか。
///
/// [`PROBE_MAX_DRIFT`] が返ったら「いくら先行しても壊れない」の意味
/// （ループ長がスロット数以下なら、小節とスロットが 1 対 1 なのでそうなる）。
fn headroom_measures(loop_measures: usize) -> usize {
    (0..=PROBE_MAX_DRIFT)
        .take_while(|drift| wrong_measures(&play(loop_measures, 32, *drift)).is_empty())
        .count()
        .saturating_sub(1)
}

/// **これが Stage 5 の芯。** 実演奏のループ長では、先行 3 小節まで正しい小節が鳴る。
///
/// 実測の先行は 1.3 小節（BPM113 で 2.7 秒）だったので、3 小節ぶんの余裕は
/// その 2 倍以上にあたる。
///
/// **スロットを 2 本へ戻すとこのテストは落ちる**（余裕が 1 小節になるため）。
/// 落ち方は実測とまったく同じで、`meas1` の位置に `meas3` が出る
/// （[`two_slots_with_a_two_measure_drift_reproduce_the_measured_swap`]）。
#[test]
fn a_clock_drift_of_three_measures_still_plays_the_measure_that_was_reserved() {
    for loop_measures in (SLOT_COUNT..=4 * SLOT_COUNT).step_by(SLOT_COUNT) {
        for drift in 0..=3 {
            let sounded = play(loop_measures, 32, drift);
            assert!(
                !sounded.is_empty(),
                "loop={loop_measures} drift={drift} で 1 音も鳴っていない（テストの組み立てが壊れている）"
            );
            assert_eq!(
                wrong_measures(&sounded),
                Vec::<&Sounded>::new(),
                "loop={loop_measures} 小節・先行 {drift} 小節で違う小節が鳴った"
            );
        }
        assert!(
            headroom_measures(loop_measures) >= 3,
            "loop={loop_measures} の余裕が 3 小節を切っている（スロットを減らした？）"
        );
    }
}

/// **余裕はループ長にも依る。** 折り返しでスロットが早回りするぶんだけ減る。
///
/// スロットは `小節 index % SLOT_COUNT` で決まるので、ループ長が `SLOT_COUNT` の
/// 倍数でないと、ループの折り返しで同じスロットへの書き込みが**間を空けずに**続く。
/// 実測できる形にすると:
///
/// ```text
/// 余裕 = 無制限                                （ループ長 <= SLOT_COUNT）
/// 余裕 = SLOT_COUNT - 1                        （ループ長が SLOT_COUNT の倍数）
/// 余裕 = (ループ長 % SLOT_COUNT) - 1           （それ以外）
/// ```
///
/// **ループ長が SLOT_COUNT 以下なら、そもそも踏み潰しが起きない。** 小節とスロットが
/// 1 対 1 になり、差し替えても中身が同じ小節だから。4 スロットにしたことで、
/// **1〜4 小節のループ（実演奏はここ）は先行が何秒あっても壊れなくなった。**
///
/// **`ループ長 % SLOT_COUNT == 1` のときは余裕 0。** 4 スロットなら 5・9・13 小節の
/// ループがそれにあたる（2 スロットの頃は 3 以上の奇数すべてがこれだった）。
/// この穴は本数を増やしても消えず、消すにはスロットの選び方を
/// 「小節 index」から「演奏した小節の通し番号」へ変える必要がある。
/// **見送った判断は `docs/adr/0012-live-clock-drift-is-absorbed-not-eliminated.md`。**
#[test]
fn the_headroom_shrinks_when_the_loop_length_is_not_a_multiple_of_the_slot_count() {
    let expected = |loop_measures: usize| {
        if loop_measures <= SLOT_COUNT {
            // 小節とスロットが 1 対 1。差し替えても中身が同じなので踏み潰しようがない。
            PROBE_MAX_DRIFT
        } else if loop_measures.is_multiple_of(SLOT_COUNT) {
            SLOT_COUNT - 1
        } else {
            loop_measures % SLOT_COUNT - 1
        }
    };
    let measured: Vec<(usize, usize)> = (1..=12)
        .map(|loop_measures| (loop_measures, headroom_measures(loop_measures)))
        .collect();

    assert_eq!(
        measured,
        (1..=12)
            .map(|loop_measures| (loop_measures, expected(loop_measures)))
            .collect::<Vec<_>>(),
        "余裕の出かたが変わった。ループ長と SLOT_COUNT の関係を読み直すこと"
    );
}

/// 実測の再現。**2 スロットで先行 2 小節だと、`meas1` の位置に `meas3` が鳴る。**
///
/// 資料の事実 2 の表そのもの（meas1↔meas3 / meas2↔meas4 が入れ替わる）。
/// スロット数を変数にせず 2 で書いてあるので、`SLOT_COUNT` を増やしても
/// **「あのとき何が起きたか」は残る。**
#[test]
fn two_slots_with_a_two_measure_drift_reproduce_the_measured_swap() {
    // 2 スロットのときの載せ先。`measure_slot` は現在の SLOT_COUNT を見るので、
    // ここだけは当時の式を直接書く。
    let slot_of = |measure: usize| measure % 2;
    let mut slots: [Option<usize>; 2] = [None, None];
    let mut sounded = Vec::new();
    for step in 0..8usize {
        if step == 0 {
            slots[slot_of(0)] = Some(0);
        }
        if let Some(fired) = step.checked_sub(2) {
            let intended = fired % 4;
            sounded.push((intended, slots[slot_of(intended)]));
        }
        let next = (step + 1) % 4;
        slots[slot_of(next)] = Some(next);
    }

    assert_eq!(
        sounded,
        vec![
            // 予約は meas1 / meas2 / meas3 / meas4（0 始まり）だが、鳴ったのは…
            (0, Some(2)),
            (1, Some(3)),
            (2, Some(0)),
            (3, Some(1)),
            (0, Some(2)),
            (1, Some(3)),
        ],
        "meas1↔meas3 / meas2↔meas4 の入れ替わり（資料の事実 2）が出るはず"
    );
}

/// **Stage 6 の芯。実演奏のループ長 5 は余裕 0 で、先行 1 小節で必ず違う小節が鳴る。**
///
/// 実演奏がこの穴に当たっていることは実ログで確定している
/// （`docs/adr/0018-page-replacement-clears-the-cache.md` の「塞いでいない穴」）
/// （実ログの `meas5: live-cache slot=0 ... next=meas1/slot0`）。
/// `5 % SLOT_COUNT == 1` なので **meas5 と meas1 が続けて同じスロット 0 を使う。**
///
/// 壊れ方は 1 種類だけ。**meas5 の予約位置で meas1 が鳴る**（逆は起きない）。頻度は
/// **5 小節に 1 回**。ここを数で固定しておくと、直したときにこの本数が 0 へ落ちる。
///
/// **「ループ長 5 だから必ず壊れる」ではない。** 先行が 0 小節なら 1 音も壊れない。
/// つまり実害が出るかどうかは、**サーバーのクロックがどれだけ遅れるか**しだいで、
/// そこはこの決定的テストでは決まらない（実測は
/// `python scripts/capture_daw_live_mix.py --loop-measures 5` の `margin`）。
#[test]
fn a_five_measure_loop_plays_measure_one_where_measure_five_was_reserved() {
    assert_eq!(
        headroom_measures(5),
        0,
        "ループ長 5 の余裕は 0 小節のはず（5 % {SLOT_COUNT} - 1）"
    );

    // 先行 0 小節: 12 音とも予約どおり。
    let in_time = play(5, 12, 0);
    assert_eq!(in_time.len(), 12, "テストの組み立てが壊れている");
    assert_eq!(
        wrong_measures(&in_time),
        Vec::<&Sounded>::new(),
        "先行が無ければループ長 5 でも壊れない"
    );

    // 先行 1 小節: 11 音のうち 2 音（＝5 小節に 1 回）で meas1 が meas5 の位置に出る。
    let slipped = play(5, 12, 1);
    assert_eq!(slipped.len(), 11, "テストの組み立てが壊れている");
    let wrong: Vec<(usize, Option<usize>)> = wrong_measures(&slipped)
        .iter()
        .map(|s| (s.intended, s.actual))
        .collect();
    assert_eq!(
        wrong,
        vec![(4, Some(0)), (4, Some(0))],
        "meas5（index 4）の予約位置で meas1（index 0）が鳴るはず。\
         他の小節は無事で、頻度はループ 1 周につき 1 回"
    );
}

/// 「その小節から演奏を始めた」ときに、**最初の 1 小節が実際に何を鳴らすか。**
///
/// 演奏開始の 1 小節目（と AB リピート・小節数が演奏中に変わった小節）は
/// `preload=miss` の経路を通る。[`super::super::LiveCachePlayLoop::run`] の順序は
///
/// 1. その小節をスロットへ載せる
/// 2. note on を **`lead` ぶん先**（`MAX_LEAD` = 250ms、または小節長の半分）へ予約する
/// 3. **その場で**次の小節を載せる
///
/// で、3 が 2 と同じスロットを指していると、**note on がまだ鳴っていないうちに
/// 中身が入れ替わる。** 定常ループ（`preload=hit`）では note on が先に鳴ってから
/// 載せ替えるので、ここだけ順序が逆になる。
fn first_measure_after_a_miss(loop_measures: usize, start: usize) -> Sounded {
    let mut slots: Vec<Option<usize>> = vec![None; SLOT_COUNT];
    slots[measure_slot(start)] = Some(start);
    // note on はここで予約されるだけ。鳴るのは `lead` ぶんあと。
    let next = following_measure_index(start, loop_measures, None);
    slots[measure_slot(next)] = Some(next);
    Sounded {
        intended: start,
        actual: slots[measure_slot(start)],
    }
}

/// **Stage 6 の実害。ループ長 5 の末尾の小節から演奏を始めると、meas1 が鳴る。**
///
/// 実アプリの入口は「カーソルの小節から演奏」（`input/normal.rs` の
/// `start_play_from_cursor_measure`）。ループ長 5 でカーソルが meas5 に在るときの
/// 1 音目がこれに当たる。
///
/// **実サーバーで再現済み**（`docs/adr/0018-page-replacement-clears-the-cache.md`）:
///
/// ```text
/// python scripts/capture_daw_live_mix.py --loop-measures 5 --start-measure 5
///   margin = -9440 frames（-0.20 秒）   <-- 踏み潰し
///   録れた波形の照合  meas5 の corr=0.05（他の小節は 0.66〜0.77）
/// ```
///
/// 定常ループ（`--start-measure 1`）では起きない。そちらの実測 margin は
/// **+2080〜+2847 frames（43〜59ms）**で、薄いが正。
#[test]
fn a_five_measure_loop_started_from_its_last_measure_sounds_the_first_measure_instead() {
    assert_eq!(
        first_measure_after_a_miss(5, 4),
        Sounded {
            intended: 4,
            actual: Some(0),
        },
        "meas5 から演奏を始めると、その 1 音目は meas1 の中身で鳴る"
    );

    // 末尾以外から始めるぶんには無事。**「ループ長 5 なら常に壊れる」ではない。**
    for start in 0..4 {
        assert_eq!(
            first_measure_after_a_miss(5, start).actual,
            Some(start),
            "ループ長 5・meas{} から開始したときまで壊れてはいない",
            start + 1
        );
    }

    // ループ長がスロット数の倍数なら、どの小節から始めても無事。
    for start in 0..SLOT_COUNT {
        assert_eq!(
            first_measure_after_a_miss(SLOT_COUNT, start).actual,
            Some(start),
            "ループ長 {SLOT_COUNT}・meas{} から開始",
            start + 1
        );
    }
}
