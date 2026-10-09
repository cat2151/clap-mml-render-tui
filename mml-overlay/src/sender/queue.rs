//! worker が受け取る列。
//!
//! 列には command と先読みの 2 種類が並ぶ。**command は新しいものが古いものを捨てる**
//! （準備待ちの間に積まれた操作は、最後の 1 つだけが意味を持つ）。**先読みはその対象に
//! しない。** 先読みで直前の再生を捨てると、頼んだ行が鳴らなくなる。

use std::sync::mpsc;

use super::live_patch::LivePatch;
use super::step_loop::StepLoopEdit;
use super::{log_line, SenderCommand};

pub(super) enum WorkerMessage {
    Command(SenderCommand),
    /// 鳴っていない bank へこの音色を読んでおく。
    Preload(LivePatch),
    /// 走っている打点ループを変える。
    StepLoop(StepLoopEdit),
}

/// 列から取り出したもの。
pub(super) struct Drained {
    /// 最新の 1 つ。
    pub(super) command: Option<SenderCommand>,
    /// 積まれた順に全部。
    pub(super) preloads: Vec<LivePatch>,
    /// 最後の command より後に積まれた、ループへの変更を積まれた順に全部。command より前の
    /// 変更は、その command（新しいループ）の内容より古いので捨てる。
    pub(super) step_edits: Vec<StepLoopEdit>,
}

/// 列に溜まっているものを全部取り出す。
pub(super) fn drain_queue(received: WorkerMessage, rx: &mpsc::Receiver<WorkerMessage>) -> Drained {
    let mut command: Option<SenderCommand> = None;
    let mut preloads = Vec::new();
    let mut step_edits = Vec::new();
    let mut next = Some(received);
    while let Some(message) = next.take().or_else(|| rx.try_recv().ok()) {
        match message {
            WorkerMessage::Command(newer) => {
                if let Some(older) = &command {
                    log_line(format!(
                        "action=mml-overlay-command event=superseded command_id={} \
                         by_command_id={}",
                        older.id, newer.id
                    ));
                }
                command = Some(newer);
                step_edits.clear();
            }
            WorkerMessage::Preload(patch) => preloads.push(patch),
            WorkerMessage::StepLoop(edit) => step_edits.push(edit),
        }
    }
    Drained {
        command,
        preloads,
        step_edits,
    }
}

#[cfg(test)]
mod tests;
