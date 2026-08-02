use std::time::Duration;

use rodio::Source as _;

use crate::LoopPlaybackClip;
use cmrt_loop_browser_domain::time_stretch::PreparedAudio;

pub fn append_clip_source(
    sink: &rodio::Sink,
    audio: &PreparedAudio,
    clip: &LoopPlaybackClip,
    playback_duration: Duration,
) -> usize {
    let info = audio.info();
    let output_frames = scheduled_clip_output_frames(
        clip,
        info.output_frames,
        info.sample_rate,
        playback_duration,
    );
    if clip.is_one_shot() {
        sink.append(audio.source());
    } else {
        sink.append(audio.source().take_duration(playback_duration));
    }
    output_frames
}

pub fn scheduled_clip_output_frames(
    clip: &LoopPlaybackClip,
    prepared_frames: usize,
    sample_rate: u32,
    playback_duration: Duration,
) -> usize {
    if clip.is_one_shot() {
        prepared_frames
    } else {
        scheduled_output_frames(prepared_frames, sample_rate, playback_duration)
    }
}

pub fn scheduled_output_frames(
    prepared_frames: usize,
    sample_rate: u32,
    playback_duration: Duration,
) -> usize {
    let limit = (playback_duration.as_secs_f64() * f64::from(sample_rate)).round();
    if !limit.is_finite() || limit <= 0.0 {
        return 0;
    }
    prepared_frames.min(limit as usize)
}

pub fn scheduled_idle_seconds(
    output_frames: usize,
    sample_rate: u32,
    playback_duration: Duration,
) -> f64 {
    if sample_rate == 0 {
        return 0.0;
    }
    (playback_duration.as_secs_f64() - output_frames as f64 / f64::from(sample_rate)).max(0.0)
}
