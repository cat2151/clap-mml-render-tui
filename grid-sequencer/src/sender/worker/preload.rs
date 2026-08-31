//! 待機 bank への先読みを「受付」と「完了待ち」に分けて持つ状態機械。
//!
//! protocol v10 で先読みは 2 段階になった（`realtime-play` の
//! `live_ipc/standby_request.rs`）。受付が返った時点でサーバー側のロードはまだ
//! 走っているので、**送信スレッドはここで待ってはいけない**。待つと同じスレッドが
//! timeline event を送れなくなり、鳴っている音の note-off がロード時間ぶん遅れる
//! （16分音符が全音符まで伸びる、が実際に観測された症状）。
//!
//! ここが持つのは「いま完了を待っている1件」と「その後ろで受付を待っている件」だけ。
//! 実際に待つ代わりに、コマンドループの毎周回で [`PreloadTracker::advance`] を呼ぶ。
//!
//! ## 世代（generation）
//!
//! 先読みサイクルは `r` キー・画面離脱・live edit で途中で畳まれる。wire 上の要求は
//! 取り消せないので、畳んだ後にも古い完了通知が届く。それを新しいサイクルの進捗として
//! 数えると、実際にはロードしていない bank へ切り替わってしまう。そこで要求を出した
//! ときの世代を持ち回り、決着時の世代と違えば [`PreloadOutcome::stale`] を立てて
//! 進捗を進めない（完了通知そのものは受け取って drain する）。

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use super::GridSenderBackend;

/// 先読み1件の決着。
///
/// `stale` が立っているものは**進捗にも失敗にも数えない**。ログにだけ残す。
pub(in crate::sender) struct PreloadOutcome {
    pub(in crate::sender) instance_id: u8,
    /// wire 上の request ID。受付にも至らなかった場合は `None`。
    pub(in crate::sender) request_id: Option<u32>,
    /// 受付から決着までの実測。受付に失敗した場合は受付にかかった時間。
    pub(in crate::sender) elapsed: Duration,
    pub(in crate::sender) error: Option<String>,
    /// 要求を出した先読みサイクルがもう畳まれている。
    pub(in crate::sender) stale: bool,
}

struct InFlight<R> {
    request: R,
    instance_id: u8,
    request_id: u32,
    generation: u64,
    started: Instant,
}

/// まだ受付へ出していない1件。完了通知 slot は共有メモリ上に1件ぶんしか無いので、
/// 受付は同時に1件しか持てない。
struct Waiting {
    generation: u64,
    instance_id: u8,
    patch: Option<String>,
}

pub(super) struct PreloadTracker<R> {
    in_flight: Option<InFlight<R>>,
    waiting: VecDeque<Waiting>,
}

impl<R> PreloadTracker<R> {
    pub(super) fn new() -> Self {
        Self {
            in_flight: None,
            waiting: VecDeque::new(),
        }
    }

    /// 先読みを1件受け付ける。**ロードの完了を待たない。**
    ///
    /// `generation` は要求を出した側（UI スレッド）の世代、`current` は今の世代。
    /// 通常は同じで、サイクルが畳まれた後に届いたコマンドだけがずれる。
    pub(super) fn submit<B: GridSenderBackend<Standby = R>>(
        &mut self,
        backend: &mut B,
        generation: u64,
        current: u64,
        instance_id: u8,
        patch: Option<String>,
    ) -> Vec<PreloadOutcome> {
        self.waiting.push_back(Waiting {
            generation,
            instance_id,
            patch,
        });
        self.advance(backend, current)
    }

    /// 完了通知を非 blocking に見て、席が空いたら次の1件を受付へ出す。
    ///
    /// 何も起きていなければ空の [`Vec`] を返す（`Vec::new()` は確保しない）。
    pub(super) fn advance<B: GridSenderBackend<Standby = R>>(
        &mut self,
        backend: &mut B,
        current: u64,
    ) -> Vec<PreloadOutcome> {
        let mut outcomes = Vec::new();
        loop {
            if let Some(outcome) = self.poll_in_flight(backend, current) {
                outcomes.push(outcome);
            }
            if self.in_flight.is_some() {
                break;
            }
            let Some(next) = self.waiting.pop_front() else {
                break;
            };
            if next.generation != current {
                // 要求元のサイクルはもう畳まれている。受付にすら行かない。
                outcomes.push(PreloadOutcome {
                    instance_id: next.instance_id,
                    request_id: None,
                    elapsed: Duration::ZERO,
                    error: None,
                    stale: true,
                });
                continue;
            }
            let started = Instant::now();
            match backend.begin_standby(next.instance_id, next.patch.as_deref()) {
                Ok(request) => {
                    let request_id = backend.standby_request_id(&request);
                    self.in_flight = Some(InFlight {
                        request,
                        instance_id: next.instance_id,
                        request_id,
                        generation: next.generation,
                        started,
                    });
                    break;
                }
                Err(error) => outcomes.push(PreloadOutcome {
                    instance_id: next.instance_id,
                    request_id: None,
                    elapsed: started.elapsed(),
                    error: Some(format!("{error:#}")),
                    stale: false,
                }),
            }
        }
        outcomes
    }

    /// 先読みを畳む。停止・画面離脱・シャットダウン用。
    ///
    /// **wire 上の要求は取り消せない。** 受付済みの1件は backend へ「もう自分のもの
    /// ではない」と伝えて手放す。クライアント側が完了通知を drain するので、次の
    /// 受付が完了 slot の取り合いで詰まることはない。
    pub(super) fn cancel<B: GridSenderBackend<Standby = R>>(
        &mut self,
        backend: &mut B,
    ) -> Vec<PreloadOutcome> {
        let mut outcomes = self
            .waiting
            .drain(..)
            .map(|waiting| PreloadOutcome {
                instance_id: waiting.instance_id,
                request_id: None,
                elapsed: Duration::ZERO,
                error: None,
                stale: true,
            })
            .collect::<Vec<_>>();
        if let Some(entry) = self.in_flight.take() {
            let outcome = PreloadOutcome {
                instance_id: entry.instance_id,
                request_id: Some(entry.request_id),
                elapsed: entry.started.elapsed(),
                error: None,
                stale: true,
            };
            backend.abandon_standby(entry.request);
            outcomes.push(outcome);
        }
        outcomes
    }

    /// 受付済みの1件を1回だけポーリングする。**block しない。**
    fn poll_in_flight<B: GridSenderBackend<Standby = R>>(
        &mut self,
        backend: &mut B,
        current: u64,
    ) -> Option<PreloadOutcome> {
        let entry = self.in_flight.as_mut()?;
        let error = match backend.poll_standby(&mut entry.request) {
            // まだロード中。呼び出し元はふつうの仕事を続ける。
            Ok(None) => return None,
            Ok(Some(())) => None,
            Err(error) => Some(format!("{error:#}")),
        };
        let entry = self
            .in_flight
            .take()
            .expect("the in flight entry was just observed");
        Some(PreloadOutcome {
            instance_id: entry.instance_id,
            request_id: Some(entry.request_id),
            elapsed: entry.started.elapsed(),
            error,
            stale: entry.generation != current,
        })
    }
}
