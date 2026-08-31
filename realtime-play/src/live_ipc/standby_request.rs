//! 先読み 1 件を追う request token と、その状態遷移。
//!
//! protocol v10 で先読みは「受付（汎用応答）」と「完了（専用 slot）」の 2 段階に
//! 分かれた。受付が返った時点でロードはまだ走っているので、呼び出し元は token を
//! 持って完了をポーリングする。
//!
//! token の寿命・timeout・二重確定の禁止は実サーバー無しで確かめられるので、
//! 状態遷移は [`StandbyPatchRequest::settle`] という 1 つの純粋な関数へ寄せてある。
//! 共有メモリからの読み取りは `fast_midi_ipc/windows/standby.rs` の担当。

use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};

use crate::{
    fast_midi_ipc::{FastIpcError, InstanceId},
    logging::log_realtime_play_event,
    RealtimePlayServerSupervisor,
};

/// 同期 wrapper が完了通知を見に行く間隔。
///
/// 完了は共有メモリの片方向 slot に載るだけなので、イベント待ちにはできない
/// （サーバーは `SetEvent` も叩くが、それは「何か変わった」ヒントに過ぎず、
/// 取り逃すと永久に待つ）。**この wrapper を使うのは grid 以外の呼び出し元だけ**で、
/// grid sequencer は待たずに自分の loop から poll する。
const STANDBY_POLL_INTERVAL: Duration = Duration::from_millis(1);

/// 先読みのログ接頭辞。start / accepted / completed / error / timeout / abandoned を
/// `request=` で突き合わせられるよう、1 か所に固めてある。
/// サーバー側の `cmrt-standby-patch: request=N` と同じ ID が出る。
const STANDBY_ACTION: &str = "shm-standby-patch-prepare";

/// 完了通知を諦めるまでの時間。
///
/// 実測の重い patch load は 3 秒前後（Surge の cold は 3.2 秒）。プラグインを
/// またぐ差し替えや予備インスタンスの背景生成待ちが重なるともう少し伸びる。
/// **ここで待ちたいのではなく、サーバーが黙って死んだときに Loading 表示を
/// 畳むための最後の安全弁**なので、実運用より充分長く取る。
/// サーバーが落ちたと分かる場合は timeout を待たず `ServerStopped` で畳まれる。
pub const STANDBY_LOAD_TIMEOUT: Duration = Duration::from_secs(60);

/// 先読み 1 件の受付票。
///
/// `Clone` にしていないのは、同じ request を 2 か所から確定させないため。
/// wire 上の同時 1 件制約は `FastMidiClient` 側の見張りが担当する。
#[derive(Debug)]
pub struct StandbyPatchRequest {
    instance_id: InstanceId,
    request_id: u32,
    since_sequence: u64,
    started: Instant,
    deadline: Instant,
    settled: bool,
}

impl StandbyPatchRequest {
    pub(super) fn new(
        instance_id: InstanceId,
        request_id: u32,
        since_sequence: u64,
        now: Instant,
        timeout: Duration,
    ) -> Self {
        Self {
            instance_id,
            request_id,
            since_sequence,
            started: now,
            deadline: now + timeout,
            settled: false,
        }
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }

    /// wire 上の request ID。サーバーの `cmrt-standby-patch: request=N` と対応する。
    pub fn request_id(&self) -> u32 {
        self.request_id
    }

    /// 完了通知の採用基準になる sequence。要求を送る **前** に読んだ値。
    pub(super) fn since_sequence(&self) -> u64 {
        self.since_sequence
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// もう成否が確定したか。確定した token をもう一度ポーリングしないこと。
    pub fn is_settled(&self) -> bool {
        self.settled
    }

    /// 共有メモリから読んだ結果（`None` はまだロード中）を状態遷移へ通す。
    ///
    /// 戻り値が `Some` になった時点でこの token は確定し、以後の呼び出しは
    /// エラーになる。**この関数が block することはない**。呼び出し元は `None` の
    /// 間ふつうの仕事（timeline event の送信など）を続ければよい。
    ///
    /// timeout の判定をここでするのは、共有メモリを読む側に時計を持ち込まないため。
    pub(super) fn settle(
        &mut self,
        snapshot: Option<Result<(), FastIpcError>>,
        now: Instant,
    ) -> Option<Result<(), FastIpcError>> {
        if self.settled {
            // 確定済みの token を回し続けるのは呼び出し元の状態管理の壊れ。
            // 黙って `None` を返すと Loading のまま残るので、必ず気づける形で返す。
            return Some(Err(FastIpcError::RequestFailed(format!(
                "standby request {} was already settled",
                self.request_id
            ))));
        }
        if let Some(result) = snapshot {
            self.settled = true;
            return Some(result);
        }
        if now >= self.deadline {
            self.settled = true;
            return Some(Err(FastIpcError::ResponseTimeout));
        }
        None
    }
}

impl RealtimePlayServerSupervisor {
    /// 非演奏 bank への先読みを要求し、**受付までで**戻る。
    ///
    /// 「この instance は鳴っている bank に属さない」という宣言を伴う専用コマンド。
    /// サーバーはそれを根拠に、その bank のレンダーを止めてロードできる。
    /// **発音 deadline を越えて非演奏になった待機 bank にだけ送ること。**
    /// 現在 bank へ送ると、鳴っている音が止まりうる。
    ///
    /// **戻った時点でロードはまだ走っている。** 完了は返した token を
    /// [`Self::poll_standby_patch`] へ渡してポーリングする。ロード中も
    /// timeline event を送り続けられるのがこの分離の目的そのものなので、
    /// 呼び出し元は token を状態として持ち、自分の loop を止めないこと。
    ///
    /// 完了通知 slot は 1 件ぶんしかないので、同時に 2 件は始められない。
    /// 前の token を [`Self::poll_standby_patch`] で確定させるか
    /// [`Self::abandon_standby_patch`] で捨ててから呼ぶこと。
    pub fn begin_standby_patch(
        &self,
        instance_id: InstanceId,
        patch: Option<&str>,
    ) -> Result<StandbyPatchRequest> {
        log_realtime_play_event(format!(
            "action={STANDBY_ACTION} event=start instance={instance_id} patch={patch:?}"
        ));
        let started = Instant::now();
        match self.with_fast_client(|client| client.begin_standby_patch(instance_id, patch)) {
            Ok((request_id, since_sequence)) => {
                log_realtime_play_event(format!(
                    "action={STANDBY_ACTION} event=accepted request={request_id} \
                     instance={instance_id} elapsed_ms={} patch={patch:?}",
                    started.elapsed().as_millis()
                ));
                Ok(StandbyPatchRequest::new(
                    instance_id,
                    request_id,
                    since_sequence,
                    started,
                    STANDBY_LOAD_TIMEOUT,
                ))
            }
            Err(error) => {
                log_realtime_play_event(format!(
                    "action={STANDBY_ACTION} event=error instance={instance_id} \
                     elapsed_ms={} patch={patch:?} error=\"{}\"",
                    started.elapsed().as_millis(),
                    crate::logging::truncate_for_log(&format!("{error:#}"), 1_000)
                ));
                Err(error)
            }
        }
    }

    /// 先読みの完了通知を**非 blocking に**読む。
    ///
    /// - `Ok(None)`: まだロード中。呼び出し元はふつうの仕事を続ける
    /// - `Ok(Some(()))`: ロード成功
    /// - `Err`: ロード失敗 / timeout / サーバー停止
    ///
    /// `fast_client` の錠は共有メモリを 1 回読む間しか握らない。ここが長く握ると、
    /// 同じ錠で timeline event を送っている側が止まり、この Stage の目的が壊れる。
    /// 未接続なら再接続せずその場で畳む（別の接続では先読みを引き継げない）。
    pub fn poll_standby_patch(&self, request: &mut StandbyPatchRequest) -> Result<Option<()>> {
        let snapshot = self.read_standby_completion(request);
        match request.settle(snapshot, Instant::now()) {
            None => Ok(None),
            Some(Ok(())) => {
                log_realtime_play_event(format!(
                    "action={STANDBY_ACTION} event=completed request={} instance={} elapsed_ms={}",
                    request.request_id(),
                    request.instance_id(),
                    request.elapsed().as_millis()
                ));
                Ok(Some(()))
            }
            Some(Err(error)) => {
                let event = if error == FastIpcError::ResponseTimeout {
                    "timeout"
                } else {
                    "error"
                };
                log_realtime_play_event(format!(
                    "action={STANDBY_ACTION} event={event} request={} instance={} \
                     elapsed_ms={} error=\"{}\"",
                    request.request_id(),
                    request.instance_id(),
                    request.elapsed().as_millis(),
                    crate::logging::truncate_for_log(&format!("{error}"), 1_000)
                ));
                Err(anyhow!(error))
            }
        }
    }

    /// 結果を捨てて、この先読みを「自分のもの」ではなくする。
    ///
    /// wire 上の要求は取り消せない。サーバーはこの後も完了通知を publish するし、
    /// 次の [`Self::begin_standby_patch`] はその完了を読んで畳んでから通る。
    /// cycle の取り消しや画面の抜け直しで、古い完了を新しい先読みの成功として
    /// 数えないための後始末。
    pub fn abandon_standby_patch(&self, request: StandbyPatchRequest) {
        if request.is_settled() {
            return;
        }
        if let Some(client) = self.fast_client.lock().unwrap().as_mut() {
            client.abandon_standby_patch(request.request_id());
        }
        log_realtime_play_event(format!(
            "action={STANDBY_ACTION} event=abandoned request={} instance={} elapsed_ms={}",
            request.request_id(),
            request.instance_id(),
            request.elapsed().as_millis()
        ));
    }

    /// 先読みをロード完了まで待つ同期 wrapper。
    ///
    /// **grid sequencer からは使わないこと。** ロードの間ずっとこのスレッドが
    /// 止まるので、grid の sender がここで待つと timeline event の供給が途切れる
    /// （それが v9 までの症状そのもの）。先読み以外の用途と、実サーバー統合テストの
    /// ように「終わるまで待ちたい」場所のためだけに残してある。
    pub fn prepare_standby_patch(
        &self,
        instance_id: InstanceId,
        patch: Option<&str>,
    ) -> Result<()> {
        let mut request = self.begin_standby_patch(instance_id, patch)?;
        loop {
            if self.poll_standby_patch(&mut request)?.is_some() {
                return Ok(());
            }
            std::thread::sleep(STANDBY_POLL_INTERVAL);
        }
    }

    /// 完了 slot を 1 回だけ読む。**錠を握るのはこの関数の中だけ。**
    fn read_standby_completion(
        &self,
        request: &StandbyPatchRequest,
    ) -> Option<Result<(), FastIpcError>> {
        let mut client = self.fast_client.lock().unwrap();
        let snapshot = match client.as_mut() {
            Some(client) => {
                client.poll_standby_completion(request.request_id(), request.since_sequence())
            }
            None => Some(Err(FastIpcError::ServerStopped)),
        };
        if matches!(
            snapshot,
            Some(Err(
                FastIpcError::ServerStopped | FastIpcError::ProtocolMismatch
            ))
        ) {
            *client = None;
            *self.fast_underrun_reader.lock().unwrap() = None;
        }
        snapshot
    }
}

#[cfg(test)]
mod tests;
