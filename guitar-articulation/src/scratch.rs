use std::ops::RangeInclusive;

use crate::fold_pitch;

/// ピックスクレイプ（`Pick_Scratch`）の sample が在る音高。30 が原音で、上ほど速く明るく擦る。
pub const PICK_SCRATCH_PITCHES: RangeInclusive<u8> = 30..=42;

/// MML の音高を、オクターブ単位で [`PICK_SCRATCH_PITCHES`] へ畳んだ音高。
pub fn pick_scratch_pitch(pitch: u8) -> u8 {
    fold_pitch(pitch, PICK_SCRATCH_PITCHES)
}

#[cfg(test)]
mod tests;
