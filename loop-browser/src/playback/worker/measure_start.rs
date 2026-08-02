//! 1 小節ぶんの clip を sink へ載せて鳴らし始める。
//!
//! ここで扱うのは既にメモリ上にある準備済み PCM だけなので、デコードやストレッチは走らない。

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;

use super::super::diagnostics::{log_event, path_label};
use super::super::gain::effective_track_gain;
use super::super::preparation::PreparedSet;
use super::super::sinks::TrackSink;
use super::super::tempo::measure_duration;
use super::super::LoopPlaybackGrid;
use crate::LoopPlaybackClip;
use cmrt_loop_browser_domain::time_stretch::format_bpm;

pub fn starting_clips(grid: &LoopPlaybackGrid, measure: usize) -> Vec<(usize, &LoopPlaybackClip)> {
    grid.iter()
        .enumerate()
        .filter_map(|(track, cells)| {
            cells
                .get(measure)
                .and_then(Option::as_ref)
                .map(|clip| (track, clip))
        })
        .collect()
}

pub fn start_measure(
    handle: &rodio::OutputStreamHandle,
    prepared: &PreparedSet,
    measure: usize,
    track_volumes_db: &[i32],
    solo_tracks: &[bool],
) -> Result<Vec<TrackSink>> {
    let clips = starting_clips(&prepared.grid, measure);
    let mut sinks = Vec::with_capacity(clips.len());
    for (track, clip) in clips {
        let Some(Ok(audio)) = prepared.audio_for(clip) else {
            log_event(format!(
                "event=playback-skip generation={} measure={} track={} path=\"{}\" reason=prepared-audio-unavailable",
                prepared.generation,
                measure + 1,
                track + 1,
                path_label(&clip.path),
            ));
            continue;
        };
        let sink = Arc::new(rodio::Sink::try_new(handle)?);
        sink.set_volume(effective_track_gain(track, track_volumes_db, solo_tracks));
        let info = audio.info();
        let measure_duration = measure_duration(&prepared.grid, prepared.target_bpm.bpm);
        let playback_duration = measure_duration
            .checked_mul(u32::try_from(clip.span_measures).unwrap_or(u32::MAX))
            .unwrap_or(Duration::MAX);
        let output_frames =
            super::super::scheduling::append_clip_source(&sink, audio, clip, playback_duration);
        let output_seconds =
            super::super::diagnostics::duration_seconds(output_frames, info.sample_rate);
        let scheduled_idle_seconds = super::super::scheduling::scheduled_idle_seconds(
            output_frames,
            info.sample_rate,
            playback_duration,
        );
        log_event(format!(
            "event=playback-start generation={} measure={} track={} path=\"{}\" source_bpm={} target_bpm={} span_measures={} measure_seconds={:.6} retrigger_after_seconds={:.6} output_frames={} output_seconds={:.6} scheduled_idle_seconds={:.6} cropped={} profile={}",
            prepared.generation,
            measure + 1,
            track + 1,
            path_label(&clip.path),
            clip.source_bpm()
                .map(format_bpm)
                .unwrap_or_else(|| "one-shot".to_string()),
            format_bpm(prepared.target_bpm.bpm),
            clip.span_measures,
            measure_duration.as_secs_f64(),
            playback_duration.as_secs_f64(),
            output_frames,
            output_seconds,
            scheduled_idle_seconds,
            output_frames < info.output_frames,
            super::super::diagnostics::profile_label(info.profile),
        ));
        sinks.push(TrackSink { track, sink });
    }
    Ok(sinks)
}
