use super::*;
use crate::GridSequencerScreen;

fn screen() -> GridSequencerScreen {
    GridSequencerScreen::with_track_count(None, 4)
}

/// 接続が `Ready` へ戻るまではステップを進めない。進めてしまうと、鳴らせない
/// 間のステップが溜まり、復帰時にまとめて鳴る。
#[test]
fn the_wait_blocks_until_the_connection_is_ready() {
    let now = Instant::now();
    let mut screen = screen();
    screen.wait_for_patches();

    assert!(!screen.poll_start_wait(now, false));
    assert!(!screen.poll_start_wait(now + PATCH_SETTLE_GUARD * 10, false));
}

/// `Ready` へ戻ってもすぐには鳴らさない。state をロードしただけの Surge XT へ
/// note on を送ると最初の1打が握り潰される。
#[test]
fn the_wait_adds_a_settle_guard_after_the_connection_is_ready() {
    let now = Instant::now();
    let mut screen = screen();
    screen.wait_for_patches();

    assert!(
        !screen.poll_start_wait(now, true),
        "Ready 直後はまだ鳴らさない"
    );
    assert!(!screen.poll_start_wait(now + PATCH_SETTLE_GUARD / 2, true));
    assert!(screen.poll_start_wait(now + PATCH_SETTLE_GUARD, true));
}

/// 待ちが明けたらクロックを step 0 から張り直す。これで再開直後の小節の頭から
/// 和音が鳴る（旧実装のように1小節ぶん和音が抜けない）。
#[test]
fn the_clock_restarts_from_the_first_step() {
    let now = Instant::now();
    let mut screen = screen();
    screen.state.start(now);
    screen.wait_for_patches();

    // 猶予は Ready を最初に見た時点から数える。
    assert!(!screen.poll_start_wait(now, true));
    assert!(screen.poll_start_wait(now + PATCH_SETTLE_GUARD, true));

    assert!(screen.state.is_running());
    assert_eq!(screen.state.step_index(), 0);
}

/// 待ちが明けたあとは素通しになり、接続状態をそのまま返す。
#[test]
fn the_wait_is_transparent_once_it_is_over() {
    let now = Instant::now();
    let mut screen = screen();
    screen.wait_for_patches();
    screen.poll_start_wait(now, true);
    assert!(screen.poll_start_wait(now + PATCH_SETTLE_GUARD, true));

    assert!(screen.poll_start_wait(now + PATCH_SETTLE_GUARD, true));
    assert!(!screen.poll_start_wait(now + PATCH_SETTLE_GUARD, false));
}

/// ロードが2回続いたら猶予も測り直す。1回目の `Ready` で始めた猶予を
/// そのまま使うと、2回目のロード中に待ちが明けてしまう。
#[test]
fn a_second_load_restarts_the_settle_guard() {
    let now = Instant::now();
    let mut screen = screen();
    screen.wait_for_patches();
    screen.poll_start_wait(now, true);

    // 別のロードが始まって Ready から外れた。
    assert!(!screen.poll_start_wait(now + PATCH_SETTLE_GUARD / 2, false));

    let back = now + PATCH_SETTLE_GUARD;
    assert!(!screen.poll_start_wait(back, true), "猶予は測り直す");
    assert!(screen.poll_start_wait(back + PATCH_SETTLE_GUARD, true));
}
