//! Chord Chart 用の複数 instance one-shot command。

use std::{
    sync::{atomic::AtomicU64, Mutex},
    time::{Duration, Instant},
};

use crate::line_play::LinePerformance;

use super::{
    is_superseded, log_superseded_after_load, sink::SoundSink, status::MmlOverlayLinePlayback,
    status::MmlOverlaySenderStatus, voice::Voice, MML_OVERLAY_INSTANCE,
};

/// 1 本の live timeline に載せる、instance ごとの one-shot performance。
#[derive(Clone, Debug, PartialEq)]
pub struct LineLayer {
    pub instance_id: u8,
    pub patch: Option<String>,
    pub performance: LinePerformance,
}

pub(super) fn play_layered_command(
    voice: &mut Voice,
    sink: &impl SoundSink,
    status: &Mutex<MmlOverlaySenderStatus>,
    command_id: u64,
    latest_command_id: &AtomicU64,
    mut layers: Vec<LineLayer>,
) {
    layers.retain(|layer| !layer.performance.is_silent());
    if layers.is_empty() {
        voice.play_layers(sink, &[]);
        return;
    }

    let mut playable = Vec::with_capacity(layers.len());
    let mut prepare_error = None;
    let mut chord_failed = false;
    for layer in layers {
        if is_superseded(command_id, latest_command_id) {
            log_superseded_after_load(command_id, latest_command_id);
            return;
        }
        if voice.is_patch_ready(layer.instance_id, layer.patch.as_deref()) {
            playable.push(layer);
            continue;
        }
        set_loading(status, &layer);
        let result = voice.prepare(sink, layer.instance_id, layer.patch.as_deref());
        clear_loading(status);
        match result {
            Ok(()) => playable.push(layer),
            Err(error) => {
                prepare_error.get_or_insert(error);
                if layer.instance_id == MML_OVERLAY_INSTANCE {
                    chord_failed = true;
                    playable.clear();
                    break;
                }
            }
        }
    }
    status.lock().unwrap().prepare_error = prepare_error;

    if is_superseded(command_id, latest_command_id) {
        log_superseded_after_load(command_id, latest_command_id);
        return;
    }
    if chord_failed || playable.is_empty() {
        voice.stop(sink, "layers-not-ready");
        return;
    }

    let duration_seconds = playable
        .iter()
        .map(|layer| layer.performance.loop_seconds)
        .fold(0.0_f64, f64::max);
    let played = voice.play_layers(sink, &playable);
    if played && !is_superseded(command_id, latest_command_id) {
        publish_playback(status, command_id, duration_seconds);
    } else if played {
        log_superseded_after_load(command_id, latest_command_id);
    }
}

fn set_loading(status: &Mutex<MmlOverlaySenderStatus>, layer: &LineLayer) {
    let mut status = status.lock().unwrap();
    status.loading = true;
    status.loading_patch = layer.patch.clone();
    status.sounding.clear();
}

fn clear_loading(status: &Mutex<MmlOverlaySenderStatus>) {
    let mut status = status.lock().unwrap();
    status.loading = false;
    status.loading_patch = None;
}

fn publish_playback(
    status: &Mutex<MmlOverlaySenderStatus>,
    command_id: u64,
    duration_seconds: f64,
) {
    let started_at = Instant::now();
    let Some(ends_at) = Duration::try_from_secs_f64(duration_seconds)
        .ok()
        .and_then(|duration| started_at.checked_add(duration))
    else {
        return;
    };
    status.lock().unwrap().line_playback = Some(MmlOverlayLinePlayback {
        command_id,
        started_at,
        ends_at: Some(ends_at),
    });
}
