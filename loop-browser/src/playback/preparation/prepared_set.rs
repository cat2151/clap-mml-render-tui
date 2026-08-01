//! 準備済みオーディオの持ち物。
//!
//! `AudioKey` は 1 回の準備の中で clip を引くためのキー、`CacheKey` はそれに
//! ファイルの実体（サイズと更新時刻）を足した、準備をまたいで使い回すためのキー。

use std::collections::HashMap;
use std::fs::Metadata;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use rubberband_ffi::StretchProfile;

use crate::{LoopPlaybackClip, LoopPlaybackGrid};
use cmrt_loop_browser_domain::time_stretch::{profile_for_category, PreparedAudio, TargetBpm};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AudioKey {
    path: PathBuf,
    bpm_bits: Option<u64>,
    target_bpm_bits: u64,
    profile: StretchProfile,
}

impl AudioKey {
    pub fn new(clip: &LoopPlaybackClip, target_bpm: f64) -> Self {
        Self {
            path: clip.path.clone(),
            bpm_bits: clip.source_bpm().map(f64::to_bits),
            target_bpm_bits: target_bpm.to_bits(),
            profile: profile_for_category(clip.category.as_deref()),
        }
    }
}

pub type PreparedEntry = Result<Arc<PreparedAudio>, Arc<str>>;

pub struct PreparedSet {
    pub generation: u64,
    pub grid: LoopPlaybackGrid,
    pub target_bpm: TargetBpm,
    pub audio: HashMap<AudioKey, PreparedEntry>,
    pub warning: Option<String>,
}

impl PreparedSet {
    pub fn audio_for(&self, clip: &LoopPlaybackClip) -> Option<&PreparedEntry> {
        self.audio.get(&AudioKey::new(clip, self.target_bpm.bpm))
    }
}

#[derive(Eq, Hash, PartialEq)]
pub struct CacheKey {
    audio: AudioKey,
    file_len: u64,
    modified_nanos: Option<u128>,
}

impl CacheKey {
    pub fn new(audio: &AudioKey, metadata: Option<&Metadata>) -> Self {
        Self {
            audio: audio.clone(),
            file_len: metadata.map_or(0, Metadata::len),
            modified_nanos: metadata
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos()),
        }
    }
}
