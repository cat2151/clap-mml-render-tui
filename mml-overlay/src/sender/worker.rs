//! sender の worker thread。列を受け取り、[`Voice`] を通して音源へ送る。

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, RecvTimeoutError},
        Arc, Mutex,
    },
    time::Instant,
};

use super::layers;
use super::live_patch::LivePatch;
use super::prepare::{prepare_if_needed, prepare_line_if_needed};
use super::queue::{drain_queue, Drained, WorkerMessage};
use super::sink::SoundSink;
use super::sounding_lines::SoundingLines;
use super::status::{
    begin_status, publish_line_playback, publish_preload, publish_step_loop, MmlOverlaySenderStatus,
};
use super::voice::{Voice, Wake};
use super::{log_line, SenderCommandKind, MML_OVERLAY_INSTANCE};

pub(super) fn run_sender<S: SoundSink + Send + Sync + 'static>(
    rx: mpsc::Receiver<WorkerMessage>,
    sink: Arc<S>,
    sample_rate_hz: f64,
    latest_command_id: Arc<AtomicU64>,
    shutting_down: Arc<AtomicBool>,
    status: Arc<Mutex<MmlOverlaySenderStatus>>,
    sounding_lines: SoundingLines,
) {
    let mut voice = Voice::new(sample_rate_hz, sounding_lines, shutting_down);
    loop {
        let received = match voice.next_wake(Instant::now()) {
            Some((wake, wait)) => match rx.recv_timeout(wait) {
                Ok(command) => command,
                // 待ちが切れた。次の操作は来ていないので、起きた理由のほうを片づける。
                Err(RecvTimeoutError::Timeout) => {
                    match wake {
                        Wake::Gate => {
                            log_line(format!(
                                "action=mml-overlay-gate-expired command_id={}",
                                status.lock().unwrap().command_id
                            ));
                            voice.stop(&*sink, "gate");
                            status.lock().unwrap().sounding.clear();
                        }
                        // 継ぎ足しは止めない。ここで stop を通すと毎周継ぎ目が出る。
                        Wake::Repeat => voice.pump_repeat(&*sink, Instant::now()),
                        Wake::Preload => {
                            voice.poll_preload(&*sink);
                            publish_preload(&status, &voice);
                        }
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(command) => command,
                Err(_) => break,
            },
        };
        let Drained {
            command,
            preloads,
            step_edits,
        } = drain_queue(received, &rx);
        voice.poll_preload(&*sink);
        let Some(command) = command else {
            request_preloads(&mut voice, &*sink, &status, preloads);
            for edit in step_edits {
                voice.edit_step_loop(&*sink, edit);
            }
            continue;
        };
        let name = command.kind.name();
        let queue_ms = command.queued_at.elapsed().as_millis();
        let started_at = Instant::now();
        log_line(format!(
            "action=mml-overlay-command event=start command_id={} command={name} queue_ms={queue_ms}",
            command.id
        ));
        voice.begin_command(command.id);
        let shutdown = matches!(&command.kind, SenderCommandKind::Shutdown);
        begin_status(&status, command.id);
        match command.kind {
            SenderCommandKind::Prepare { patch } => {
                // 同じ patch が既に ready でも、prepare は「音源を明け渡して準備する」
                // 操作なので、前の line playback は必ず止める。直前の Stop command は
                // newest_queued_command でこの Prepare に畳み込まれうるため、ここ自身が
                // stop の意味を持つ必要がある。
                voice.stop(&*sink, "prepare");
                prepare_if_needed(&mut voice, &*sink, &status, MML_OVERLAY_INSTANCE, &patch);
            }
            SenderCommandKind::PlayNotes {
                patch,
                messages,
                gate,
            } => {
                let ready =
                    prepare_if_needed(&mut voice, &*sink, &status, MML_OVERLAY_INSTANCE, &patch);
                if ready && !is_superseded(command.id, &latest_command_id) {
                    if voice.play_notes(&*sink, &messages, gate) {
                        status.lock().unwrap().sounding = note_on_pitches(&messages);
                    }
                } else if ready {
                    log_superseded_after_load(command.id, &latest_command_id);
                }
            }
            SenderCommandKind::PlayLine {
                patch,
                program,
                stop_before_prepare,
            } => {
                if stop_before_prepare {
                    voice.stop(&*sink, "replace-line");
                }
                let ready = prepare_line_if_needed(&mut voice, &*sink, &status, &patch);
                if ready && !is_superseded(command.id, &latest_command_id) {
                    let played = voice.play_line(&*sink, &program);
                    if played && !is_superseded(command.id, &latest_command_id) {
                        publish_line_playback(&status, command.id, &program);
                    } else if played {
                        log_superseded_after_load(command.id, &latest_command_id);
                    }
                } else if ready {
                    log_superseded_after_load(command.id, &latest_command_id);
                }
            }
            SenderCommandKind::PlayStepLoop { patch, step_loop } => {
                voice.stop(&*sink, "replace-line");
                let ready = prepare_line_if_needed(&mut voice, &*sink, &status, &patch);
                if ready && !is_superseded(command.id, &latest_command_id) {
                    let origin = voice.play_step_loop(&*sink, step_loop);
                    if let (Some(origin), false) =
                        (origin, is_superseded(command.id, &latest_command_id))
                    {
                        publish_step_loop(&status, command.id, origin);
                    } else if origin.is_some() {
                        log_superseded_after_load(command.id, &latest_command_id);
                    }
                } else if ready {
                    log_superseded_after_load(command.id, &latest_command_id);
                }
            }
            SenderCommandKind::PlayLayers { layers } => layers::play_layered_command(
                &mut voice,
                &*sink,
                &status,
                command.id,
                &latest_command_id,
                layers,
            ),
            SenderCommandKind::Stop => voice.stop(&*sink, "stop"),
            SenderCommandKind::Supersede => {}
            SenderCommandKind::Shutdown => {
                voice.stop(&*sink, "shutdown");
                voice.abandon_preload(&*sink);
            }
        }
        // 音色の先読みとループへの変更は、同じ列でその前に積まれた command の後に効かせる。
        if !shutdown {
            request_preloads(&mut voice, &*sink, &status, preloads);
            for edit in step_edits {
                voice.edit_step_loop(&*sink, edit);
            }
        }
        log_line(format!(
            "action=mml-overlay-command event=finished command_id={} command={name} \
             elapsed_ms={}",
            command.id,
            started_at.elapsed().as_millis()
        ));
        if shutdown {
            break;
        }
    }
}

fn request_preloads(
    voice: &mut Voice,
    sink: &impl SoundSink,
    status: &Mutex<MmlOverlaySenderStatus>,
    preloads: Vec<LivePatch>,
) {
    for patch in preloads {
        voice.request_preload(sink, patch);
    }
    publish_preload(status, voice);
}

pub(super) fn is_superseded(command_id: u64, latest_command_id: &AtomicU64) -> bool {
    latest_command_id.load(Ordering::Acquire) > command_id
}

pub(super) fn log_superseded_after_load(command_id: u64, latest_command_id: &AtomicU64) {
    log_line(format!(
        "action=mml-overlay-command event=superseded-after-load command_id={command_id} \
         by_command_id={}",
        latest_command_id.load(Ordering::Acquire)
    ));
}

fn note_on_pitches(messages: &[[u8; 3]]) -> Vec<u8> {
    messages
        .iter()
        .filter(|message| message[0] == crate::NOTE_ON && message[2] > 0)
        .map(|message| message[1])
        .collect()
}
