//! standby patch load の「完了通知」slot の読み取り。
//!
//! 汎用 request/response（[`super::protocol::ResponseSlot`]）は **受付応答** 専用に
//! なった。standby patch のロードは秒単位かかることがあり、その完了を汎用応答で
//! 待つとサーバーの受信ループが塞がって live timeline の MIDI が届かなくなる。
//! そこでロード結果だけを、サーバーが一方的に書きこちらがポーリングで読む片方向
//! slot へ分離してある。
//!
//! 同期は [`SharedRing::standby_sequence`] の seqlock ただ 1 つ。
//!
//! - publish（サーバー）: `sequence` を奇数にする → body を書く → 偶数にする
//! - read（こちら）: 偶数の `before` を読む → body を読む → `after` が `before` と
//!   同じなら採用
//!
//! body は [`std::cell::UnsafeCell`] で atomic ordering を持たないので、この順序が
//! 唯一の保証である。サーバー側 (`realtime-ipc/src/windows/standby.rs`) と必ず
//! 同じ規約にすること。

use std::sync::atomic::Ordering;

use super::{
    protocol::{
        SharedRing, StandbyCompletionSlot, KIND_PREPARE_STANDBY_PATCH, STANDBY_STATUS_ERROR,
        STANDBY_STATUS_SUCCESS,
    },
    FastIpcError, FastMidiClient, InstanceId, MAX_STANDBY_ERROR_BYTES,
};

/// 書き換え中の body を読んでしまったときに諦めるまでの再試行回数。
///
/// publish は 1 KiB 程度の memcpy なので、この回数で足りなければ待つより
/// 呼び出し元へ「まだ」と返した方がよい。read 側は決して block しない。
const TORN_READ_RETRIES: usize = 64;

/// これから出す standby request の「基準 sequence」。
///
/// 完了通知は `sequence > watermark` のものだけを自分のものとして採用する。
/// request ID だけで判定すると、ID が wrap したときに過去の完了を自分の成功として
/// 拾ってしまう。sequence は単調増加なのでその取り違えが起きない。
///
/// 読んだ値が奇数（= publish 実行中）なら、その publish はこの request より前に
/// 始まっているので自分のものではありえない。偶数へ切り上げて「過去」に含める。
fn standby_watermark(ring: &SharedRing) -> u64 {
    let sequence = ring.standby_sequence.load(Ordering::Acquire);
    (sequence + 1) & !1
}

/// `request_id` の完了通知を非 blocking に読む。
///
/// - `None`: まだ完了していない / 自分より前の古い完了 / 書き換え中
/// - `Some(Ok(()))`: ロード成功
/// - `Some(Err(_))`: ロード失敗、または slot が壊れている
///
/// 呼び出し元は `None` の間ポーリングを続ける。ここで待たないことがこの分離の
/// 目的そのものなので、block させないこと。
fn read_standby_completion(
    ring: &SharedRing,
    request_id: u32,
    since_sequence: u64,
) -> Option<Result<(), FastIpcError>> {
    for _ in 0..TORN_READ_RETRIES {
        let before = ring.standby_sequence.load(Ordering::Acquire);
        if before & 1 != 0 {
            // publish 実行中。body は読まない。
            std::hint::spin_loop();
            continue;
        }
        if before == 0 {
            // まだ一度も publish されていない。
            return None;
        }
        // SAFETY: seqlock の read 側。`before` と `after` が一致したときだけ採用する。
        let snapshot = unsafe { StandbySnapshot::read(&*ring.standby.get()) };
        if ring.standby_sequence.load(Ordering::Acquire) != before {
            std::hint::spin_loop();
            continue;
        }
        if before <= since_sequence || snapshot.request_id != request_id {
            return None;
        }
        return Some(snapshot.into_result());
    }
    None
}

struct StandbySnapshot {
    request_id: u32,
    status: u32,
    payload_len: u32,
    payload: Vec<u8>,
}

impl StandbySnapshot {
    /// body をそのままコピーする。`payload_len` が壊れていても panic しないよう、
    /// 切り出す長さは必ず上限で clamp する。長さの検証は sequence 確認の後に行う。
    fn read(slot: &StandbyCompletionSlot) -> Self {
        let len = (slot.payload_len as usize).min(MAX_STANDBY_ERROR_BYTES);
        Self {
            request_id: slot.request_id,
            status: slot.status,
            payload_len: slot.payload_len,
            payload: slot.payload[..len].to_vec(),
        }
    }

    fn into_result(self) -> Result<(), FastIpcError> {
        if self.payload_len as usize > MAX_STANDBY_ERROR_BYTES {
            return Err(FastIpcError::InvalidPayload(
                "standby completion payload length is invalid".into(),
            ));
        }
        match self.status {
            STANDBY_STATUS_SUCCESS => Ok(()),
            STANDBY_STATUS_ERROR => Err(FastIpcError::RequestFailed(
                String::from_utf8_lossy(&self.payload).into_owned(),
            )),
            _ => Err(FastIpcError::InvalidPayload(
                "standby completion status is invalid".into(),
            )),
        }
    }
}

/// 先読み要求の「受付」と「完了」を分けたクライアント API。
///
/// ここを [`super`] 本体から分けてあるのは、standby だけが 2 段階の要求だからで、
/// 他の patch 系要求（`prepare_patch` / `probe_patch`）は今も汎用応答 1 回で終わる。
impl FastMidiClient {
    /// 先読みを要求し、**受付応答まで**待って request token の中身を返す。
    ///
    /// 返るのは `(request_id, since_sequence)`。ロードの完了は返らないので、
    /// 呼び出し元はこの 2 つを持って [`Self::poll_standby_completion`] を回す。
    ///
    /// `since_sequence`（watermark）は **要求を push する前に**読む。後に読むと、
    /// 送った直後に publish された自分の完了を「自分より前のもの」として捨ててしまう。
    ///
    /// 「対象 instance は鳴っている bank に属さない」という宣言を伴うので、
    /// 現在 bank の行音色変更・MML overlay・起動時 prepare には使わないこと。
    pub fn begin_standby_patch(
        &mut self,
        instance_id: InstanceId,
        patch: Option<&str>,
    ) -> Result<(u32, u64), FastIpcError> {
        self.standby.claim(self.mapping.ring())?;
        let since_sequence = standby_watermark(self.mapping.ring());
        let (request_id, _) =
            self.patch_request_with_id(KIND_PREPARE_STANDBY_PATCH, instance_id, patch)?;
        self.standby.started(request_id, since_sequence);
        Ok((request_id, since_sequence))
    }

    /// standby patch load の完了通知を非 blocking に読む。
    ///
    /// `None` はまだロード中。汎用応答（受付応答）とは別経路なので、
    /// ここを何度呼んでもコマンド送信は塞がらないし、ここが待つこともない。
    ///
    /// サーバーが落ちていれば `Some(Err(ServerStopped))` を返す。slot はもう
    /// 更新されないので、これを見ないと呼び出し元は timeout まで待つことになる。
    pub fn poll_standby_completion(
        &mut self,
        request_id: u32,
        since_sequence: u64,
    ) -> Option<Result<(), FastIpcError>> {
        if let Err(error) = self.check_server_alive() {
            self.standby.release(request_id);
            return Some(Err(error));
        }
        let result = read_standby_completion(self.mapping.ring(), request_id, since_sequence);
        if result.is_some() {
            self.standby.release(request_id);
        }
        result
    }

    /// 結果を捨てて、この request を「自分のもの」ではなくする。
    ///
    /// wire 上の要求は取り消せない。サーバーはこの後も完了通知を publish するし、
    /// 次の [`Self::begin_standby_patch`] はその完了を読んで畳んでから通る。
    /// ここでするのは呼び出し元側の後始末だけ。
    pub fn abandon_standby_patch(&mut self, request_id: u32) {
        self.standby.release(request_id);
    }

    /// 進行中の standby request の ID。無ければ `None`。
    pub fn standby_in_flight(&self) -> Option<u32> {
        self.standby.active_request_id()
    }
}

/// 「standby request は同時に 1 件だけ」をクライアント側で守る見張り。
///
/// 完了通知 slot は共有メモリ上に 1 件ぶんしか無い。2 件を同時に走らせると
/// 後から publish された方が前を上書きし、前の要求は永久に Loading のまま残る
/// （不変条件 8 の違反）。サーバーも 2 件目を断るが、こちらで先に弾けば
/// 往復ぶんの無駄と、断られ方の解釈の揺れが消える。
#[derive(Default)]
pub(super) struct StandbyInFlight {
    active: Option<ActiveStandby>,
}

#[derive(Clone, Copy)]
struct ActiveStandby {
    request_id: u32,
    since_sequence: u64,
}

impl StandbyInFlight {
    fn active_request_id(&self) -> Option<u32> {
        self.active.map(|active| active.request_id)
    }

    /// 新しい要求を出してよいか確かめ、よければ席を空ける。
    ///
    /// 前の要求の token を持ったまま呼び出し元が消えても、その完了がもう slot に
    /// 載っていれば黙って畳んで通す（[`Self::release`] を呼び忘れても永久に
    /// 詰まらせない）。まだ載っていなければ拒否する。
    fn claim(&mut self, ring: &SharedRing) -> Result<(), FastIpcError> {
        let Some(active) = self.active else {
            return Ok(());
        };
        if read_standby_completion(ring, active.request_id, active.since_sequence).is_none() {
            return Err(FastIpcError::InvalidPayload(format!(
                "standby patch load {} is still in flight",
                active.request_id
            )));
        }
        self.active = None;
        Ok(())
    }

    fn started(&mut self, request_id: u32, since_sequence: u64) {
        self.active = Some(ActiveStandby {
            request_id,
            since_sequence,
        });
    }

    /// `request_id` が今の要求なら席を空ける。古い要求の後始末では何も起きない。
    fn release(&mut self, request_id: u32) {
        if self.active_request_id() == Some(request_id) {
            self.active = None;
        }
    }
}

/// サーバーの publish 手順をそのまま写したテスト用の書き手。
///
/// 本番でこちらが completion を書くことはない。それでもここに置いてあるのは、
/// read 側の契約を実サーバー無しで固定するためと、publish の順序（odd -> body ->
/// even）を TUI 側にも残して 2 repository のずれに気づけるようにするため。
#[cfg(test)]
fn publish_standby_completion(ring: &SharedRing, request_id: u32, result: Result<(), &str>) -> u64 {
    let (status, message) = match result {
        Ok(()) => (STANDBY_STATUS_SUCCESS, ""),
        Err(message) => (STANDBY_STATUS_ERROR, message),
    };
    let payload = message.as_bytes();
    assert!(payload.len() <= MAX_STANDBY_ERROR_BYTES);
    ring.standby_sequence.fetch_add(1, Ordering::AcqRel);
    unsafe {
        let slot = &mut *ring.standby.get();
        slot.request_id = request_id;
        slot.status = status;
        slot.payload_len = payload.len() as u32;
        slot.payload[..payload.len()].copy_from_slice(payload);
    }
    ring.standby_sequence.fetch_add(1, Ordering::Release) + 1
}

#[cfg(test)]
mod tests;
