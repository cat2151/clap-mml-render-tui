use super::*;

/// 共有メモリを張らずに seqlock の規約だけを検証する。
/// レイアウトは `protocol.rs` の size assertion が別途固定している。
fn new_ring() -> Box<SharedRing> {
    // SAFETY: `SharedRing` は repr(C) で、全 0 はサーバーが mapping を初期化した
    // 直後の状態そのもの。
    unsafe { Box::new(std::mem::zeroed::<SharedRing>()) }
}

fn set_body(ring: &SharedRing, request_id: u32, status: u32, payload_len: u32) {
    unsafe {
        let slot = &mut *ring.standby.get();
        slot.request_id = request_id;
        slot.status = status;
        slot.payload_len = payload_len;
    }
}

#[test]
fn unpublished_slot_reports_no_completion() {
    let ring = new_ring();
    let watermark = standby_watermark(&ring);
    assert_eq!(watermark, 0);
    assert!(read_standby_completion(&ring, 1, watermark).is_none());
}

#[test]
fn success_is_visible_only_after_the_watermark() {
    let ring = new_ring();
    let watermark = standby_watermark(&ring);
    assert_eq!(publish_standby_completion(&ring, 7, Ok(())), 2);
    assert_eq!(read_standby_completion(&ring, 7, watermark), Some(Ok(())));
    // ポーリング前提なので、読んでも消費されない。
    assert_eq!(read_standby_completion(&ring, 7, watermark), Some(Ok(())));
}

#[test]
fn error_message_is_carried_by_the_completion_slot() {
    let ring = new_ring();
    let watermark = standby_watermark(&ring);
    publish_standby_completion(&ring, 3, Err("patch not found: Keys/Missing.fxp"));
    assert_eq!(
        read_standby_completion(&ring, 3, watermark),
        Some(Err(FastIpcError::RequestFailed(
            "patch not found: Keys/Missing.fxp".into()
        )))
    );
}

#[test]
fn completion_for_another_request_id_is_ignored() {
    let ring = new_ring();
    let watermark = standby_watermark(&ring);
    publish_standby_completion(&ring, 41, Ok(()));
    assert!(read_standby_completion(&ring, 42, watermark).is_none());
}

/// request ID は u32 で wrap する。ID だけで判定すると、前回 cycle の完了を
/// 今回の成功として拾ってしまう。watermark がそれを防ぐ。
#[test]
fn completion_published_before_the_watermark_is_not_reused_after_id_wrap() {
    let ring = new_ring();
    publish_standby_completion(&ring, 9, Ok(()));
    let watermark = standby_watermark(&ring);
    assert!(read_standby_completion(&ring, 9, watermark).is_none());

    publish_standby_completion(&ring, 9, Ok(()));
    assert_eq!(read_standby_completion(&ring, 9, watermark), Some(Ok(())));
}

/// publish 中（sequence が奇数）の body は決して採用しない。
#[test]
fn torn_write_is_never_observed_as_a_completion() {
    let ring = new_ring();
    let watermark = standby_watermark(&ring);
    ring.standby_sequence.fetch_add(1, Ordering::AcqRel);
    set_body(&ring, 5, STANDBY_STATUS_SUCCESS, 0);
    assert!(read_standby_completion(&ring, 5, watermark).is_none());

    ring.standby_sequence.fetch_add(1, Ordering::Release);
    assert_eq!(read_standby_completion(&ring, 5, watermark), Some(Ok(())));
}

/// 奇数のまま watermark を取ると、その publish は「自分より前」に含まれる。
#[test]
fn watermark_rounds_an_in_flight_publish_into_the_past() {
    let ring = new_ring();
    ring.standby_sequence.fetch_add(1, Ordering::AcqRel);
    set_body(&ring, 5, STANDBY_STATUS_SUCCESS, 0);
    let watermark = standby_watermark(&ring);
    assert_eq!(watermark, 2);
    ring.standby_sequence.fetch_add(1, Ordering::Release);
    assert!(read_standby_completion(&ring, 5, watermark).is_none());
}

#[test]
fn corrupt_payload_length_fails_the_request_instead_of_hanging() {
    let ring = new_ring();
    let watermark = standby_watermark(&ring);
    publish_standby_completion(&ring, 2, Ok(()));
    set_body(
        &ring,
        2,
        STANDBY_STATUS_ERROR,
        MAX_STANDBY_ERROR_BYTES as u32 + 1,
    );
    assert!(matches!(
        read_standby_completion(&ring, 2, watermark),
        Some(Err(FastIpcError::InvalidPayload(_)))
    ));
}

#[test]
fn unknown_status_fails_the_request_instead_of_hanging() {
    let ring = new_ring();
    let watermark = standby_watermark(&ring);
    publish_standby_completion(&ring, 2, Ok(()));
    set_body(&ring, 2, 99, 0);
    assert!(matches!(
        read_standby_completion(&ring, 2, watermark),
        Some(Err(FastIpcError::InvalidPayload(_)))
    ));
}

/// エラーメッセージの上限は 2 repository で同じであること。サーバー側が
/// この長さで切り詰めるので、こちらが違う値を持つと slot からはみ出す。
#[test]
fn max_error_bytes_fills_the_slot_exactly() {
    let ring = new_ring();
    let watermark = standby_watermark(&ring);
    let message = "e".repeat(MAX_STANDBY_ERROR_BYTES);
    publish_standby_completion(&ring, 4, Err(&message));
    assert_eq!(
        read_standby_completion(&ring, 4, watermark),
        Some(Err(FastIpcError::RequestFailed(message)))
    );
}

/// 「同時 1 件」の見張り。実サーバーが無くても、共有メモリの形が同じ zeroed ring
/// で全経路を通せる。
mod in_flight {
    use super::*;

    #[test]
    fn the_first_request_always_gets_the_seat() {
        let ring = new_ring();
        let mut guard = StandbyInFlight::default();
        assert_eq!(guard.claim(&ring), Ok(()));
        guard.started(1, standby_watermark(&ring));
        assert_eq!(guard.active_request_id(), Some(1));
    }

    /// 完了通知 slot は 1 件ぶんしかない。走っている間の 2 件目は必ず断る。
    #[test]
    fn a_second_request_is_refused_while_the_first_is_still_loading() {
        let ring = new_ring();
        let mut guard = StandbyInFlight::default();
        guard.claim(&ring).unwrap();
        guard.started(1, standby_watermark(&ring));
        let error = guard.claim(&ring).expect_err("2 件目が通ってしまった");
        assert!(
            matches!(&error, FastIpcError::InvalidPayload(message) if message.contains("in flight")),
            "{error:?}"
        );
    }

    /// token を持ったまま呼び出し元が消えても、完了がもう slot に載っていれば
    /// 次の要求は通る。ここが無いと `release` の呼び忘れで永久に詰まる。
    #[test]
    fn a_finished_request_is_drained_so_the_next_one_can_start() {
        let ring = new_ring();
        let mut guard = StandbyInFlight::default();
        guard.claim(&ring).unwrap();
        let watermark = standby_watermark(&ring);
        guard.started(1, watermark);
        publish_standby_completion(&ring, 1, Ok(()));
        assert_eq!(guard.claim(&ring), Ok(()));
        assert_eq!(guard.active_request_id(), None);
    }

    #[test]
    fn releasing_the_current_request_frees_the_seat() {
        let ring = new_ring();
        let mut guard = StandbyInFlight::default();
        guard.started(7, standby_watermark(&ring));
        guard.release(7);
        assert_eq!(guard.active_request_id(), None);
        assert_eq!(guard.claim(&ring), Ok(()));
    }

    /// 見捨てた古い request の後始末が、次の request の席を奪わないこと。
    #[test]
    fn releasing_an_old_request_does_not_touch_the_current_one() {
        let ring = new_ring();
        let mut guard = StandbyInFlight::default();
        guard.started(7, standby_watermark(&ring));
        guard.release(6);
        assert_eq!(guard.active_request_id(), Some(7));
    }
}
