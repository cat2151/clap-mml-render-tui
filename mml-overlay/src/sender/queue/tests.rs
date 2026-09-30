use std::{sync::mpsc, time::Instant};

use super::*;
use crate::sender::SenderCommandKind;

fn command(id: u64) -> WorkerMessage {
    WorkerMessage::Command(SenderCommand {
        id,
        queued_at: Instant::now(),
        kind: SenderCommandKind::Stop,
    })
}

fn preload(name: &str) -> WorkerMessage {
    WorkerMessage::Preload(LivePatch::new(Some(name)))
}

#[test]
fn a_preload_does_not_supersede_the_command_before_it() {
    let (tx, rx) = mpsc::channel();
    tx.send(preload("full.sfz")).unwrap();
    let (command, preloads) = drain_queue(command(1), &rx);
    assert_eq!(command.map(|command| command.id), Some(1));
    assert_eq!(preloads, vec![LivePatch::new(Some("full.sfz"))]);
}

#[test]
fn commands_still_supersede_each_other_around_a_preload() {
    let (tx, rx) = mpsc::channel();
    tx.send(preload("full.sfz")).unwrap();
    tx.send(command(2)).unwrap();
    let (command, preloads) = drain_queue(command(1), &rx);
    assert_eq!(command.map(|command| command.id), Some(2));
    assert_eq!(preloads, vec![LivePatch::new(Some("full.sfz"))]);
}

#[test]
fn a_preload_alone_has_no_command() {
    let (_tx, rx) = mpsc::channel();
    let (command, preloads) = drain_queue(preload("full.sfz"), &rx);
    assert!(command.is_none());
    assert_eq!(preloads.len(), 1);
}
