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
    let Drained {
        command, preloads, ..
    } = drain_queue(command(1), &rx);
    assert_eq!(command.map(|command| command.id), Some(1));
    assert_eq!(preloads, vec![LivePatch::new(Some("full.sfz"))]);
}

#[test]
fn commands_still_supersede_each_other_around_a_preload() {
    let (tx, rx) = mpsc::channel();
    tx.send(preload("full.sfz")).unwrap();
    tx.send(command(2)).unwrap();
    let Drained {
        command, preloads, ..
    } = drain_queue(command(1), &rx);
    assert_eq!(command.map(|command| command.id), Some(2));
    assert_eq!(preloads, vec![LivePatch::new(Some("full.sfz"))]);
}

#[test]
fn a_preload_alone_has_no_command() {
    let (_tx, rx) = mpsc::channel();
    let Drained {
        command, preloads, ..
    } = drain_queue(preload("full.sfz"), &rx);
    assert!(command.is_none());
    assert_eq!(preloads.len(), 1);
}

fn hits(note: u8) -> WorkerMessage {
    WorkerMessage::StepLoop(StepLoopEdit::Hits(vec![crate::sender::StepHit {
        seconds: 0.0,
        note,
        velocity: 127,
        gate_seconds: 0.125,
    }]))
}

fn notes(edits: &[StepLoopEdit]) -> Vec<u8> {
    edits
        .iter()
        .map(|edit| match edit {
            StepLoopEdit::Hits(hits) => hits[0].note,
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn step_hits_before_the_last_command_are_older_than_its_loop() {
    let (tx, rx) = mpsc::channel();
    tx.send(command(2)).unwrap();
    let drained = drain_queue(hits(36), &rx);
    assert_eq!(drained.command.map(|command| command.id), Some(2));
    assert!(drained.step_edits.is_empty());

    tx.send(hits(38)).unwrap();
    tx.send(hits(40)).unwrap();
    let drained = drain_queue(command(3), &rx);
    assert_eq!(drained.command.map(|command| command.id), Some(3));
    assert_eq!(notes(&drained.step_edits), vec![38, 40], "applied in order");
}
