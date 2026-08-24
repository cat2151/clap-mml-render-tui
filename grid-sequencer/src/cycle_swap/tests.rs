use cmrt_chord::ChordProgressionCatalog;

use super::*;
use crate::tests::ctx_with;
use crate::{GridPatchLoad, GridVoicingLookup, GRID_STEPS, STEP_INTERVAL};
use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::patch_load::PatchLoadMeasurement;

const CATALOG_JSON: &str = r#"[{"degrees":"I-IV-V-I","description":"test"}]"#;

fn catalog() -> ChordProgressionCatalog {
    ChordProgressionCatalog::from_json(CATALOG_JSON).unwrap()
}

fn patches() -> Vec<(String, String)> {
    vec![("Keys/Poly.fxp".to_string(), "keys/poly.fxp".to_string())]
}

#[test]
fn preload_weights_follow_the_selected_catalog_times_and_fill_gaps_with_the_median() {
    let measurements = std::collections::BTreeMap::from([
        (
            "A.vvp".to_string(),
            PatchLoadMeasurement {
                second_load_ms: Some(100),
                ..PatchLoadMeasurement::default()
            },
        ),
        (
            "B.sfz".to_string(),
            PatchLoadMeasurement {
                second_load_ms: Some(400),
                ..PatchLoadMeasurement::default()
            },
        ),
    ]);
    let selected = vec![
        (16, Some("A.vvp".to_string())),
        (17, Some("missing.fxp".to_string())),
        (18, None),
        (19, Some("B.sfz".to_string())),
    ];

    assert_eq!(
        preload_weights_ms(&selected, Some(&measurements)),
        vec![100, 250, 1, 400]
    );
}

/// すべて poly とみなす lookup。chord mode を開始できるようにするためだけのもの。
struct AllPoly;

impl GridVoicingLookup for AllPoly {
    fn cached_voicing(&self, _patch: &str) -> Option<PatchVoicing> {
        Some(PatchVoicing::Poly)
    }
}

/// 4 トラックの画面を chord mode で走らせる。MIDI は送らない。
fn screen_in_chord_mode(
    now: Instant,
    ctx: &GridSequencerContext<'_>,
) -> crate::GridSequencerScreen {
    let mut screen = crate::GridSequencerScreen::with_track_count(None, 4);
    screen.start(now, ctx);
    screen.toggle_chord_mode(now, ctx);
    screen
}

/// 進行の先頭から次サイクルを抽選し、1ステップにつき1件ずつ先読みする。
/// まとめて投げるとサーバーのレンダースレッドが連続で止まり underrun になる。
#[test]
fn the_preload_sends_one_instance_per_step() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_chord_mode(now, &ctx);

    screen.advance_cycle_swap(now, &ctx);
    assert!(screen.state.has_pending_cycle(), "次サイクルを抽選する");
    assert_eq!(sent_rows(&screen), 1, "1ステップで送るのは1件だけ");

    // `pump_step` はフレームごとに呼ばれる。1ステップ経つまでは次を送らない
    // （まとめて投げるとサーバーのレンダースレッドが連続で止まる）。
    for frame in 1..10 {
        screen.advance_cycle_swap(now + STEP_INTERVAL / 20 * frame, &ctx);
    }
    assert_eq!(sent_rows(&screen), 1, "1ステップ経つまで次は送らない");

    for step in 1..screen.state.row_count() as u32 {
        screen.advance_cycle_swap(now + STEP_INTERVAL * step, &ctx);
    }
    assert_eq!(sent_rows(&screen), screen.state.row_count());

    // 全件送り終えた次のステップで完了する。
    screen.advance_cycle_swap(now + STEP_INTERVAL * screen.state.row_count() as u32, &ctx);
    assert!(screen.cycle_swap.is_none(), "先読みは終わっている");
}

/// 音色を据え置く周は、待機 bank のロードも bank 切替も不要。
#[test]
fn keeping_the_patches_stages_the_next_progression_without_starting_a_bank_preload() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_chord_mode(now, &ctx);
    screen.cycle_random = crate::CycleRandom::HOLD;
    screen.state.stage_preload_due_for_test();

    screen.advance_cycle_swap(now, &ctx);

    assert!(screen.state.has_pending_cycle(), "次の進行は境界待ちにする");
    assert!(screen.cycle_swap.is_none(), "patch の先読みは開始しない");
    assert_eq!(screen.state.bank(), 0, "active bank はそのまま");
}

/// 譜面を据え置いて音色だけ毎周変える構成。ここが動かないと「patch だけ random」が
/// 成立しない（先読みが走らないと差し替え先 bank に音色が載らない）。
#[test]
fn randomizing_only_the_patches_still_starts_the_bank_preload() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_chord_mode(now, &ctx);
    screen.cycle_random = crate::CycleRandom {
        patch: true,
        ..crate::CycleRandom::HOLD
    };
    let before = screen.state.instances().to_vec();
    screen.state.stage_preload_due_for_test();

    screen.advance_cycle_swap(now, &ctx);

    assert!(screen.cycle_swap.is_some(), "音色の先読みは始める");
    let staged = screen.state.pending_instances_for_test();
    for (staged, before) in staged.iter().zip(&before) {
        assert_eq!(staged.lanes, before.lanes, "譜面は据え置く");
    }
}

fn sent_rows(screen: &crate::GridSequencerScreen) -> usize {
    screen
        .cycle_swap
        .as_ref()
        .map_or(0, |swap| swap.next_instance)
}

/// 先読み中は演奏している grid を書き換えない。書き換えると、まだ鳴っている
/// 小節の途中で音程やリズムが変わってしまう。
#[test]
fn the_playing_grid_is_untouched_while_preloading() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_chord_mode(now, &ctx);
    let playing = screen.state.rows().to_vec();
    screen.state.stage_preload_due_for_test();

    for step in 0..=screen.state.row_count() as u32 {
        screen.advance_cycle_swap(now + STEP_INTERVAL * step, &ctx);
    }

    assert_eq!(screen.state.rows(), playing.as_slice());
    assert_eq!(screen.state.bank(), 0, "差し替えは小節境界まで起きない");
}

/// 抽選のたびに譜面（note / pattern）も引き直す。`r` キーと同じ範囲。
#[test]
fn staging_a_cycle_rerolls_the_score_too() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_chord_mode(now, &ctx);
    // 引き直しが起きたことが分かるよう、全patternを空にしておく。
    for row in screen.state.rows_mut() {
        row.pattern = crate::NotePattern::default();
    }

    assert!(screen.stage_next_cycle(now, &ctx));

    let staged = screen.state.pending_rows_for_test();
    assert!(
        staged
            .iter()
            .any(|row| row.pattern.steps().contains(&crate::NoteStep::Attack)),
        "note patternが引き直される"
    );
}

/// 先読みが済んでいない間は次のサイクルを抽選し直さない。追い越すと、
/// ロード済みの patch と鳴らす予定の grid が食い違う。
#[test]
fn a_second_preload_does_not_overtake_the_first() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_chord_mode(now, &ctx);
    screen.state.stage_preload_due_for_test();
    screen.advance_cycle_swap(now, &ctx);
    let staged = screen.state.pending_rows_for_test();

    // もう一度合図が立っても、走っている先読みを捨てて引き直さない。
    screen.state.stage_preload_due_for_test();
    screen.advance_cycle_swap(now, &ctx);

    assert_eq!(screen.state.pending_rows_for_test(), staged);
}

/// 進行先頭で早めに抽選したあと設定を変えても、変更前の pending cycle を次周まで
/// 鳴らしてはいけない。現在の設定で直ちに仕込み直せる状態へ戻す。
#[test]
fn changing_cycle_random_restarts_the_early_preload() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_chord_mode(now, &ctx);
    screen.advance_cycle_swap(now, &ctx);
    assert!(screen.state.has_pending_cycle());
    assert!(screen.cycle_swap.is_some());

    screen.set_cycle_random(crate::CycleRandomItem::Note, false);

    assert!(!screen.state.has_pending_cycle(), "変更前の抽選を捨てる");
    assert!(screen.cycle_swap.is_none(), "変更前のロードを打ち切る");

    screen.advance_cycle_swap(now + STEP_INTERVAL, &ctx);

    assert!(screen.state.has_pending_cycle(), "変更後の設定で抽選し直す");
    assert!(screen.cycle_swap.is_some(), "待機 bank のロードを再開する");
}

/// `r` キーのように grid を丸ごと差し替えるときは、走っている先読みを捨てる。
#[test]
fn cancelling_drops_the_staged_cycle() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_chord_mode(now, &ctx);
    screen.state.stage_preload_due_for_test();
    screen.advance_cycle_swap(now, &ctx);

    screen.cancel_cycle_swap();

    assert!(screen.cycle_swap.is_none());
    assert!(!screen.state.has_pending_cycle());
}

/// chord mode を使っていない間は先読みも bank 切り替えも起きない。
#[test]
fn nothing_is_preloaded_without_the_chord_mode() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = crate::GridSequencerScreen::with_track_count(None, 4);
    screen.start(now, &ctx);

    for step in 0..(GRID_STEPS as u64 * 3) {
        screen.pump_step(now + crate::state::step_offset(step), &ctx);
    }

    assert!(screen.cycle_swap.is_none());
    assert!(!screen.state.has_pending_cycle());
    assert_eq!(screen.state.bank(), 0);
}
