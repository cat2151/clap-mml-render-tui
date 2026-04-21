use std::{
    cmp::Ordering as CmpOrdering,
    collections::BinaryHeap,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Condvar, Mutex,
    },
};

use anyhow::{anyhow, Result};
use cmrt_core::NativeRenderProbeContext;

use crate::offline_render::{OfflineRenderer, PreparedOfflineRender};

#[derive(Clone)]
pub(super) struct RenderQueue {
    request_tx: Option<mpsc::Sender<RenderRequest>>,
    next_request_id: Arc<AtomicU64>,
}

pub(super) struct RenderResult {
    pub(super) request_id: u64,
    pub(super) result: Result<Vec<f32>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RenderPriority {
    Low,
    Normal,
    High,
}

struct RenderRequest {
    request_id: u64,
    sequence: u64,
    priority: RenderPriority,
    mml: String,
    probe_context: NativeRenderProbeContext,
    response_tx: mpsc::Sender<RenderResult>,
}

struct PreparedRenderRequest {
    request: RenderRequest,
    prepared: PreparedOfflineRender,
}

struct QueuedRenderRequest(RenderRequest);

impl PartialEq for QueuedRenderRequest {
    fn eq(&self, other: &Self) -> bool {
        self.0.priority == other.0.priority && self.0.sequence == other.0.sequence
    }
}

impl Eq for QueuedRenderRequest {}

impl PartialOrd for QueuedRenderRequest {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedRenderRequest {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.0
            .priority
            .cmp(&other.0.priority)
            .then_with(|| other.0.sequence.cmp(&self.0.sequence))
    }
}

struct QueuedPreparedRenderRequest(PreparedRenderRequest);

impl PartialEq for QueuedPreparedRenderRequest {
    fn eq(&self, other: &Self) -> bool {
        self.0.request.priority == other.0.request.priority
            && self.0.request.sequence == other.0.request.sequence
    }
}

impl Eq for QueuedPreparedRenderRequest {}

impl PartialOrd for QueuedPreparedRenderRequest {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedPreparedRenderRequest {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.0
            .request
            .priority
            .cmp(&other.0.request.priority)
            .then_with(|| other.0.request.sequence.cmp(&self.0.request.sequence))
    }
}

#[derive(Default)]
struct PreparedRenderQueueState {
    pending: BinaryHeap<QueuedPreparedRenderRequest>,
    closed: bool,
}

#[derive(Default)]
struct PreparedRenderQueue {
    state: Mutex<PreparedRenderQueueState>,
    available: Condvar,
}

impl PreparedRenderQueue {
    fn push(&self, request: PreparedRenderRequest) {
        let mut state = self.state.lock().unwrap();
        if state.closed {
            return;
        }
        state.pending.push(QueuedPreparedRenderRequest(request));
        self.available.notify_one();
    }

    fn close(&self) {
        self.state.lock().unwrap().closed = true;
        self.available.notify_all();
    }

    fn pop(&self) -> Option<PreparedRenderRequest> {
        let mut state = self.state.lock().unwrap();
        loop {
            if let Some(request) = state.pending.pop() {
                return Some(request.0);
            }
            if state.closed {
                return None;
            }
            state = self.available.wait(state).unwrap();
        }
    }
}

impl RenderQueue {
    pub(super) fn new(
        cfg: Arc<crate::config::Config>,
        entry_ptr: usize,
        render_workers: usize,
    ) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<RenderRequest>();
        let prepared_queue = Arc::new(PreparedRenderQueue::default());
        let renderer = OfflineRenderer::new(Arc::clone(&cfg), entry_ptr);

        {
            let prepared_queue = Arc::clone(&prepared_queue);
            let renderer = renderer.clone();
            std::thread::spawn(move || {
                let mut pending = BinaryHeap::<QueuedRenderRequest>::new();
                loop {
                    if pending.is_empty() {
                        let Ok(request) = request_rx.recv() else {
                            prepared_queue.close();
                            break;
                        };
                        pending.push(QueuedRenderRequest(request));
                    }
                    while let Ok(request) = request_rx.try_recv() {
                        pending.push(QueuedRenderRequest(request));
                    }
                    let Some(QueuedRenderRequest(request)) = pending.pop() else {
                        continue;
                    };
                    match renderer.prepare_cache_render(&request.mml) {
                        Ok(prepared) => {
                            prepared_queue.push(PreparedRenderRequest { request, prepared });
                        }
                        Err(error) => {
                            let _ = request.response_tx.send(RenderResult {
                                request_id: request.request_id,
                                result: Err(error),
                            });
                        }
                    }
                }
            });
        }

        for _ in 0..render_workers {
            let prepared_queue = Arc::clone(&prepared_queue);
            let renderer = renderer.clone();
            std::thread::spawn(move || loop {
                let Some(prepared) = prepared_queue.pop() else {
                    break;
                };
                let result = renderer.render_prepared_cache(
                    prepared.prepared,
                    Some(&prepared.request.probe_context),
                );
                let _ = prepared.request.response_tx.send(RenderResult {
                    request_id: prepared.request.request_id,
                    result,
                });
            });
        }

        Self {
            request_tx: Some(request_tx),
            next_request_id: Arc::new(AtomicU64::new(1)),
        }
    }

    #[cfg(test)]
    pub(super) fn disabled_for_tests() -> Self {
        Self {
            request_tx: None,
            next_request_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub(super) fn reserve_request_id(&self) -> u64 {
        self.next_request_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(super) fn submit_with_id(
        &self,
        request_id: u64,
        priority: RenderPriority,
        mml: String,
        probe_context: NativeRenderProbeContext,
        response_tx: mpsc::Sender<RenderResult>,
    ) -> Result<()> {
        let Some(request_tx) = &self.request_tx else {
            return Err(anyhow!("render queue is disabled"));
        };
        request_tx
            .send(RenderRequest {
                request_id,
                sequence: request_id,
                priority,
                mml,
                probe_context,
                response_tx,
            })
            .map_err(|_| anyhow!("render queue is closed"))
    }

    pub(super) fn render_blocking(
        &self,
        priority: RenderPriority,
        mml: &str,
        probe_context: NativeRenderProbeContext,
    ) -> Result<Vec<f32>> {
        let request_id = self.reserve_request_id();
        let (response_tx, response_rx) = mpsc::channel();
        self.submit_with_id(
            request_id,
            priority,
            mml.to_string(),
            probe_context,
            response_tx,
        )?;
        response_rx
            .recv()
            .map_err(|_| anyhow!("render queue response channel is closed"))?
            .result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queued_request(request_id: u64, priority: RenderPriority) -> QueuedRenderRequest {
        let (response_tx, _response_rx) = mpsc::channel();
        QueuedRenderRequest(RenderRequest {
            request_id,
            sequence: request_id,
            priority,
            mml: String::new(),
            probe_context: NativeRenderProbeContext::tui_prefetch(0, request_id, 1),
            response_tx,
        })
    }

    #[test]
    fn render_request_queue_prefers_priority_then_fifo_order() {
        let mut pending = BinaryHeap::new();
        pending.push(queued_request(1, RenderPriority::Normal));
        pending.push(queued_request(2, RenderPriority::Low));
        pending.push(queued_request(3, RenderPriority::High));
        pending.push(queued_request(4, RenderPriority::High));

        assert_eq!(pending.pop().unwrap().0.request_id, 3);
        assert_eq!(pending.pop().unwrap().0.request_id, 4);
        assert_eq!(pending.pop().unwrap().0.request_id, 1);
        assert_eq!(pending.pop().unwrap().0.request_id, 2);
    }
}
