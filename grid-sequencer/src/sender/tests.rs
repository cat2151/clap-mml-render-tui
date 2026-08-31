//! [`super::GridMidiSender`] のテスト。
//!
//! - `preload_during_playback`: 先読み中も送信キューが詰まらないこと（実サーバー / `#[ignore]`）
//! - `test_play_server`: 実サーバーを起こすためのハーネス（判定は書かない）

mod preload_during_playback;
mod test_play_server;

use cmrt_realtime_play::LimiterMeter;

use super::*;

#[test]
fn status_starts_idle_with_no_gain_reduction() {
    let status = GridConnectionStatus::default();
    assert_eq!(status.phase, GridConnectionPhase::Idle);
    assert_eq!(status.limiter_reduction_db, 0.0);
}

#[test]
fn successful_send_updates_gain_reduction() {
    let mut status = GridConnectionStatus::default();
    status.apply_result(
        Ok(LimiterMeter {
            current_reduction_db: 1.0,
            peak_reduction_db: 3.5,
        }),
        None,
        false,
    );
    assert_eq!(status.phase, GridConnectionPhase::Ready);
    assert_eq!(status.limiter_reduction_db, 3.5);
}
