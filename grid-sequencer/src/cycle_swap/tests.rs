use cmrt_chord::ChordProgressionCatalog;

use super::*;
use crate::tests::ctx_with;
use crate::{GridPatchLoad, GridVoicingLookup, GRID_STEPS, STEP_INTERVAL};
use cmrt_realtime_play::PatchVoicing;

const CATALOG_JSON: &str = r#"[{"degrees":"I-IV-V-I","description":"test"}]"#;

fn catalog() -> ChordProgressionCatalog {
    ChordProgressionCatalog::from_json(CATALOG_JSON).unwrap()
}

fn patches() -> Vec<(String, String)> {
    vec![("Keys/Poly.fxp".to_string(), "keys/poly.fxp".to_string())]
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

/// 最終小節へ入ったら次サイクルを抽選し、1ステップにつき1件ずつ先読みする。
/// まとめて投げるとサーバーのレンダースレッドが連続で止まり underrun になる。
#[test]
fn the_preload_sends_one_instance_per_step() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_chord_mode(now, &ctx);
    // 最終小節へ入った合図を立てる。
    screen.state.stage_preload_due_for_test();

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

fn sent_rows(screen: &crate::GridSequencerScreen) -> usize {
    screen.cycle_swap.as_ref().map_or(0, |swap| swap.next_row)
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

/// 抽選のたびに譜面（note / 音長 / セル）も引き直す。`r` キーと同じ範囲。
#[test]
fn staging_a_cycle_rerolls_the_score_too() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_chord_mode(now, &ctx);
    // 引き直しが起きたことが分かるよう、全セルを落としておく。
    for row in screen.state.rows_mut() {
        row.cells = [false; GRID_STEPS];
    }

    assert!(screen.stage_next_cycle(now, &ctx));

    let staged = screen.state.pending_rows_for_test();
    assert!(
        staged.iter().any(|row| row.cells.iter().any(|cell| *cell)),
        "セルが引き直される"
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
