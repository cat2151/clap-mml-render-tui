use cmrt_chord::ChordProgressionCatalog;
use cmrt_realtime_play::PatchVoicing;

use super::*;
use crate::tests::ctx_with;
use crate::{
    step_offset, GridPatchLoad, GridScheduledMessage, GridVoicingLookup, GRID_STEPS, LOOKAHEAD,
};

/// 4 コードの進行。1サイクル = 4 小節 = 64 ステップ。
const CATALOG_JSON: &str = r#"[{"degrees":"I-IV-V-I","description":"test"}]"#;
/// 1サイクルのステップ数。
const CYCLE_STEPS: u64 = GRID_STEPS as u64 * 4;

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

/// 4 トラックの画面を、シングルバッファリングの chord mode で走らせる。MIDI は送らない。
fn screen_in_single_buffering(now: Instant, ctx: &GridSequencerContext<'_>) -> GridSequencerScreen {
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.start(now, ctx);
    screen.toggle_chord_mode(now, ctx);
    screen.toggle_single_buffering();
    // `start()` / `toggle_chord_mode()` が立てた音色ロード待ちは、MIDI を送らない
    // テストモードでは明けない。ロードが**新たに**始まったことを見たいので下ろす。
    screen.waiting_for_patches = false;
    screen
}

/// 本番の `pump_step` と同じ順で1フレームぶん進める。テストモードでは接続が Ready へ
/// ならず `poll_start_wait` が通らないので、そこだけ迂回する。
fn pump(
    screen: &mut GridSequencerScreen,
    now: Instant,
    ctx: &GridSequencerContext<'_>,
) -> Vec<GridScheduledMessage> {
    let scheduled = screen.state.poll_steps(now, LOOKAHEAD);
    screen.advance_single_buffer_cycle(now, ctx);
    scheduled
}

/// 進行を1周し終えたところでクロックを畳み、鳴っている音を止める。
#[test]
fn the_clock_folds_up_once_the_cycle_has_been_played_out() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_single_buffering(now, &ctx);

    let mut note_offs = 0;
    for step in 0..CYCLE_STEPS {
        let scheduled = pump(&mut screen, now + step_offset(step), &ctx);
        if !screen.state.is_running() {
            note_offs = scheduled
                .iter()
                .filter(|message| message.message[0] == 0x80)
                .count();
            break;
        }
    }

    assert!(!screen.state.is_running(), "1周ぶん鳴らしたら止まる");
    assert!(note_offs > 0, "鳴っていた音は止める");
    assert!(screen.cycle_end_at.is_some(), "吐き出し待ちへ入る");
}

/// 吐き出し待ちの間は音色ロードを始めない。始めるとサーバーがリングを捨て、
/// 最後の小節が尻切れになる。
#[test]
fn the_patch_load_waits_for_the_output_ring_to_drain() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_single_buffering(now, &ctx);
    let load_at = run_until_cycle_end(&mut screen, now, &ctx);

    pump(&mut screen, load_at - Duration::from_millis(1), &ctx);
    assert!(
        !screen.waiting_for_patches,
        "吐き出し待ちの間はロードしない"
    );

    pump(&mut screen, load_at, &ctx);
    assert!(screen.waiting_for_patches, "吐き出し終わったらロードへ入る");
    assert!(screen.cycle_end_at.is_none());
}

/// 差し替えは bank を動かさずに行う。裏読みしていないので待機 bank は空のまま。
#[test]
fn the_staged_cycle_is_committed_without_swapping_banks() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_single_buffering(now, &ctx);
    let load_at = run_until_cycle_end(&mut screen, now, &ctx);
    let staged = screen.state.pending_rows_for_test();
    assert!(!staged.is_empty(), "最終小節で次サイクルを抽選してある");

    pump(&mut screen, load_at, &ctx);

    assert_eq!(screen.state.rows(), staged.as_slice());
    assert_eq!(screen.state.bank(), 0, "bank は動かさない");
    assert!(!screen.state.has_pending_cycle());
}

/// chord mode を使っていない間はサイクルの区切りが無いので、何も起きない。
#[test]
fn nothing_happens_without_the_chord_mode() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.start(now, &ctx);
    screen.toggle_single_buffering();
    screen.waiting_for_patches = false;

    for step in 0..CYCLE_STEPS * 2 {
        pump(&mut screen, now + step_offset(step), &ctx);
    }

    assert!(screen.state.is_running(), "鳴らし続ける");
    assert!(screen.cycle_end_at.is_none());
    assert!(!screen.waiting_for_patches);
}

/// ダブルバッファリングへ戻したら、鳴らしきりでの停止も解ける。
#[test]
fn switching_back_to_double_buffering_disarms_the_stop() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllPoly);
    let mut screen = screen_in_single_buffering(now, &ctx);
    pump(&mut screen, now, &ctx);

    screen.toggle_single_buffering();
    assert!(!screen.single_buffering);

    for step in 1..CYCLE_STEPS + GRID_STEPS as u64 {
        let at = now + step_offset(step);
        let scheduled = screen.state.poll_steps(at, LOOKAHEAD);
        screen.send_scheduled(&scheduled);
        screen.advance_cycle_swap(at, &ctx);
    }

    assert!(screen.state.is_running(), "1周しても止まらない");
    assert!(screen.cycle_end_at.is_none());
}

/// 1サイクル鳴らしきるまで進め、音色ロードへ入る時刻を返す。
fn run_until_cycle_end(
    screen: &mut GridSequencerScreen,
    now: Instant,
    ctx: &GridSequencerContext<'_>,
) -> Instant {
    for step in 0..CYCLE_STEPS {
        pump(screen, now + step_offset(step), ctx);
        if let Some(load_at) = screen.cycle_end_at {
            return load_at;
        }
    }
    panic!("1サイクル進めても鳴らしきりに到達しなかった");
}
