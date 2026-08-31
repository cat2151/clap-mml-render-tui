//! request token の状態遷移。**共有メモリもサーバーも要らない。**
//!
//! ここで固定したいのは「受付が返っただけでは完了ではない」「一度確定した token は
//! 二度と動かない」「サーバーが黙って死んでも必ず畳まれる」の 3 つ。

use std::time::Duration;

use super::*;

fn request(now: Instant) -> StandbyPatchRequest {
    StandbyPatchRequest::new(3, 7, 42, now, Duration::from_secs(10))
}

/// 受付直後は「まだロード中」。ここが `Some` になっていたのが v9 までの設計で、
/// そのせいでロード中の timeline MIDI が止まっていた。
#[test]
fn a_fresh_request_is_still_loading() {
    let now = Instant::now();
    let mut request = request(now);
    assert_eq!(request.settle(None, now), None);
    assert!(!request.is_settled());
    assert_eq!(request.request_id(), 7);
    assert_eq!(request.instance_id(), 3);
    assert_eq!(request.since_sequence(), 42);
}

/// 受付とほぼ同時にロードが終わっても、最初のポーリングで拾えること。
/// watermark を要求の前に読んでいるので取り逃がさない。
#[test]
fn a_completion_that_arrives_immediately_is_not_missed() {
    let now = Instant::now();
    let mut request = request(now);
    assert_eq!(request.settle(Some(Ok(())), now), Some(Ok(())));
    assert!(request.is_settled());
}

#[test]
fn a_failed_load_settles_with_the_error_from_the_completion_slot() {
    let now = Instant::now();
    let mut request = request(now);
    let failure = FastIpcError::RequestFailed("patch not found".into());
    assert_eq!(
        request.settle(Some(Err(failure.clone())), now),
        Some(Err(failure))
    );
    assert!(request.is_settled());
}

/// サーバーが黙って死んだ場合。共有メモリの slot はもう更新されないので、
/// timeout を待たずに畳めること（不変条件 8）。
#[test]
fn a_stopped_server_settles_the_request_without_waiting_for_the_timeout() {
    let now = Instant::now();
    let mut request = request(now);
    assert_eq!(
        request.settle(Some(Err(FastIpcError::ServerStopped)), now),
        Some(Err(FastIpcError::ServerStopped))
    );
    assert!(request.is_settled());
}

/// 完了通知そのものが来ない場合の最後の安全弁。ここが無いと Loading が永久に残る。
#[test]
fn a_silent_load_settles_as_a_timeout_at_the_deadline() {
    let now = Instant::now();
    let mut request = request(now);
    assert_eq!(request.settle(None, now + Duration::from_secs(9)), None);
    assert_eq!(
        request.settle(None, now + Duration::from_secs(10)),
        Some(Err(FastIpcError::ResponseTimeout))
    );
    assert!(request.is_settled());
}

/// 確定済みの token を回し続けるのは呼び出し元の壊れ。黙って `None` を返すと
/// 呼び出し元が永久に待つので、必ず気づける形で返す。
#[test]
fn settling_twice_is_reported_instead_of_silently_pending() {
    let now = Instant::now();
    let mut request = request(now);
    assert_eq!(request.settle(Some(Ok(())), now), Some(Ok(())));
    assert!(matches!(
        request.settle(Some(Ok(())), now),
        Some(Err(FastIpcError::RequestFailed(message))) if message.contains("already settled")
    ));
}

/// 実運用の timeout が、実測のロード時間より充分長いこと。
/// Surge の cold load は 3.2 秒、プラグインまたぎの差し替えはそれ以上かかる。
#[test]
fn the_default_timeout_leaves_room_for_the_slowest_measured_load() {
    assert!(STANDBY_LOAD_TIMEOUT >= Duration::from_secs(30));
}
