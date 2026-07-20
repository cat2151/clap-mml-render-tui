mod raw;

use std::ptr::NonNull;

use thiserror::Error;

const BLOCK_FRAMES: usize = 16_384;

pub const GIT_REVISION: &str = env!("RUBBERBAND_GIT_REV");
pub const C_API_MAJOR_VERSION: u32 = 3;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StretchProfile {
    Drum,
    General,
}

impl StretchProfile {
    fn options(self) -> i32 {
        match self {
            Self::Drum => {
                raw::PROCESS_OFFLINE
                    | raw::ENGINE_FASTER
                    | raw::DETECTOR_PERCUSSIVE
                    | raw::TRANSIENTS_CRISP
            }
            Self::General => raw::PROCESS_OFFLINE | raw::ENGINE_FINER | raw::CHANNELS_TOGETHER,
        }
    }
}

#[derive(Debug, Error)]
pub enum StretchError {
    #[error("channel数は1以上である必要があります")]
    NoChannels,
    #[error("sample rateは1以上である必要があります")]
    InvalidSampleRate,
    #[error("sample数がchannel数で割り切れません")]
    UnalignedSamples,
    #[error("time ratioが不正です: {0}")]
    InvalidRatio(f64),
    #[error("入力がRubber Band C APIの上限を超えています")]
    InputTooLong,
    #[error("Rubber Band stretcherを作成できません")]
    CreateFailed,
    #[error("Rubber Bandの出力sample数が不正です")]
    InvalidOutput,
    #[error("Rubber Band処理をキャンセルしました")]
    Cancelled,
}

pub fn engine_version(profile: StretchProfile) -> Result<i32, StretchError> {
    let stretcher = Stretcher::new(48_000, 2, profile, 1.0)?;
    Ok(unsafe { raw::rubberband_get_engine_version(stretcher.state.as_ptr()) })
}

pub fn stretch_interleaved(
    input: &[f32],
    sample_rate: u32,
    channels: u16,
    time_ratio: f64,
    profile: StretchProfile,
) -> Result<Vec<f32>, StretchError> {
    stretch_interleaved_with_cancel(input, sample_rate, channels, time_ratio, profile, || false)
}

pub fn stretch_interleaved_with_cancel<F>(
    input: &[f32],
    sample_rate: u32,
    channels: u16,
    time_ratio: f64,
    profile: StretchProfile,
    cancelled: F,
) -> Result<Vec<f32>, StretchError>
where
    F: Fn() -> bool,
{
    if channels == 0 {
        return Err(StretchError::NoChannels);
    }
    if sample_rate == 0 {
        return Err(StretchError::InvalidSampleRate);
    }
    if !input.len().is_multiple_of(usize::from(channels)) {
        return Err(StretchError::UnalignedSamples);
    }
    if !time_ratio.is_finite() || time_ratio <= 0.0 {
        return Err(StretchError::InvalidRatio(time_ratio));
    }
    let frames = input.len() / usize::from(channels);
    let frames_u32 = u32::try_from(frames).map_err(|_| StretchError::InputTooLong)?;
    if frames == 0 {
        return Ok(Vec::new());
    }

    let planar = deinterleave(input, usize::from(channels));
    let stretcher = Stretcher::new(sample_rate, channels, profile, time_ratio)?;
    unsafe {
        raw::rubberband_set_expected_input_duration(stretcher.state.as_ptr(), frames_u32);
    }
    stretcher.feed_pass(&planar, true, &cancelled)?;
    stretcher.process(&planar, &cancelled)
}

struct Stretcher {
    state: NonNull<std::ffi::c_void>,
    channels: usize,
}

impl Stretcher {
    fn new(
        sample_rate: u32,
        channels: u16,
        profile: StretchProfile,
        time_ratio: f64,
    ) -> Result<Self, StretchError> {
        let state = unsafe {
            raw::rubberband_new(
                sample_rate,
                u32::from(channels),
                profile.options(),
                time_ratio,
                1.0,
            )
        };
        Ok(Self {
            state: NonNull::new(state).ok_or(StretchError::CreateFailed)?,
            channels: usize::from(channels),
        })
    }

    fn feed_pass<F>(
        &self,
        planar: &[Vec<f32>],
        study: bool,
        cancelled: &F,
    ) -> Result<(), StretchError>
    where
        F: Fn() -> bool,
    {
        let frames = planar.first().map_or(0, Vec::len);
        for offset in (0..frames).step_by(BLOCK_FRAMES) {
            if cancelled() {
                return Err(StretchError::Cancelled);
            }
            let count = (frames - offset).min(BLOCK_FRAMES);
            let count_u32 = u32::try_from(count).map_err(|_| StretchError::InputTooLong)?;
            let pointers = planar
                .iter()
                .map(|channel| unsafe { channel.as_ptr().add(offset) })
                .collect::<Vec<_>>();
            let final_block = i32::from(offset + count == frames);
            unsafe {
                if study {
                    raw::rubberband_study(
                        self.state.as_ptr(),
                        pointers.as_ptr(),
                        count_u32,
                        final_block,
                    );
                } else {
                    raw::rubberband_process(
                        self.state.as_ptr(),
                        pointers.as_ptr(),
                        count_u32,
                        final_block,
                    );
                }
            }
        }
        Ok(())
    }

    fn process<F>(&self, planar: &[Vec<f32>], cancelled: &F) -> Result<Vec<f32>, StretchError>
    where
        F: Fn() -> bool,
    {
        let frames = planar.first().map_or(0, Vec::len);
        let mut output = vec![Vec::<f32>::new(); self.channels];
        for offset in (0..frames).step_by(BLOCK_FRAMES) {
            if cancelled() {
                return Err(StretchError::Cancelled);
            }
            let count = (frames - offset).min(BLOCK_FRAMES);
            let count_u32 = u32::try_from(count).map_err(|_| StretchError::InputTooLong)?;
            let pointers = planar
                .iter()
                .map(|channel| unsafe { channel.as_ptr().add(offset) })
                .collect::<Vec<_>>();
            unsafe {
                raw::rubberband_process(
                    self.state.as_ptr(),
                    pointers.as_ptr(),
                    count_u32,
                    i32::from(offset + count == frames),
                );
            }
            self.retrieve_available(&mut output)?;
        }
        self.retrieve_available(&mut output)?;
        interleave(&output)
    }

    fn retrieve_available(&self, output: &mut [Vec<f32>]) -> Result<(), StretchError> {
        loop {
            let available = unsafe { raw::rubberband_available(self.state.as_ptr()) };
            if available <= 0 {
                return Ok(());
            }
            let frames = usize::try_from(available).map_err(|_| StretchError::InvalidOutput)?;
            let mut block = vec![vec![0.0; frames]; self.channels];
            let pointers = block.iter_mut().map(Vec::as_mut_ptr).collect::<Vec<_>>();
            let retrieved = unsafe {
                raw::rubberband_retrieve(
                    self.state.as_ptr(),
                    pointers.as_ptr(),
                    u32::try_from(frames).map_err(|_| StretchError::InvalidOutput)?,
                )
            } as usize;
            if retrieved == 0 || retrieved > frames {
                return Err(StretchError::InvalidOutput);
            }
            for (destination, source) in output.iter_mut().zip(block) {
                destination.extend_from_slice(&source[..retrieved]);
            }
        }
    }
}

impl Drop for Stretcher {
    fn drop(&mut self) {
        unsafe { raw::rubberband_delete(self.state.as_ptr()) };
    }
}

fn deinterleave(input: &[f32], channels: usize) -> Vec<Vec<f32>> {
    let mut output = vec![Vec::with_capacity(input.len() / channels); channels];
    for frame in input.chunks_exact(channels) {
        for (channel, sample) in output.iter_mut().zip(frame) {
            channel.push(*sample);
        }
    }
    output
}

fn interleave(input: &[Vec<f32>]) -> Result<Vec<f32>, StretchError> {
    let frames = input.first().map_or(0, Vec::len);
    if input.iter().any(|channel| channel.len() != frames) {
        return Err(StretchError::InvalidOutput);
    }
    let capacity = frames
        .checked_mul(input.len())
        .ok_or(StretchError::InvalidOutput)?;
    let mut output = Vec::with_capacity(capacity);
    for frame in 0..frames {
        for channel in input {
            output.push(channel[frame]);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_select_expected_engines() {
        assert_eq!(engine_version(StretchProfile::Drum).unwrap(), 2);
        assert_eq!(engine_version(StretchProfile::General).unwrap(), 3);
    }

    #[test]
    fn offline_stretch_changes_length_and_preserves_stereo_alignment() {
        let input = (0..4_800)
            .flat_map(|frame| {
                let sample = ((frame as f32 / 48.0) * std::f32::consts::TAU).sin();
                [sample, -sample]
            })
            .collect::<Vec<_>>();
        let output = stretch_interleaved(&input, 48_000, 2, 0.825, StretchProfile::Drum).unwrap();

        let expected_frames = (4_800.0_f64 * 0.825).round() as usize;
        assert!(output.len().abs_diff(expected_frames * 2) <= 4);
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.len().is_multiple_of(2));
        assert!(output.chunks_exact(2).any(|frame| frame[0] != 0.0));
        assert!(output.chunks_exact(2).any(|frame| frame[1] != 0.0));
    }

    #[test]
    fn validates_inputs() {
        assert!(matches!(
            stretch_interleaved(&[0.0], 48_000, 0, 1.0, StretchProfile::General),
            Err(StretchError::NoChannels)
        ));
        assert!(matches!(
            stretch_interleaved(&[0.0], 48_000, 2, 1.0, StretchProfile::General),
            Err(StretchError::UnalignedSamples)
        ));
        assert!(matches!(
            stretch_interleaved(&[0.0], 48_000, 1, f64::NAN, StretchProfile::General),
            Err(StretchError::InvalidRatio(_))
        ));
    }
}
