//! worker が受け取る列。
//!
//! 列には command と先読みの 2 種類が並ぶ。**command は新しいものが古いものを捨てる**
//! （準備待ちの間に積まれた操作は、最後の 1 つだけが意味を持つ）。**先読みはその対象に
//! しない。** 先読みで直前の再生を捨てると、頼んだ行が鳴らなくなる。

use std::sync::mpsc;

use super::live_patch::LivePatch;
use super::{log_line, SenderCommand};

pub(super) enum WorkerMessage {
    Command(SenderCommand),
    /// 鳴っていない bank へこの音色を読んでおく。
    Preload(LivePatch),
}

/// 列に溜まっているものを全部取り出す。
///
/// command は最新の 1 つ（無ければ `None`）、先読みは積まれた順に全部を返す。
pub(super) fn drain_queue(
    received: WorkerMessage,
    rx: &mpsc::Receiver<WorkerMessage>,
) -> (Option<SenderCommand>, Vec<LivePatch>) {
    let mut command: Option<SenderCommand> = None;
    let mut preloads = Vec::new();
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
            }
            WorkerMessage::Preload(patch) => preloads.push(patch),
        }
    }
    (command, preloads)
}

#[cfg(test)]
mod tests;
