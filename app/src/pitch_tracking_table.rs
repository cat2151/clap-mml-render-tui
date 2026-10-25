//! 共有の音高追従表（cat-music-patterns の main）を取り直し、取れなければ前回の cache を読む。
//! 照合は `cmrt_patches::pitch_tracking`、移し方は `cmrt_patch_select::pitch_tracking`。

use std::{path::Path, sync::Mutex};

use anyhow::{Context, Result};
use cmrt_patches::pitch_tracking::{PitchTrackingTable, PITCH_TRACKING_FILE, PITCH_TRACKING_URL};

use crate::cached_source::CachedSource;

pub(crate) fn load() -> Result<PitchTrackingTable> {
    let config_dir = crate::config::config_app_dir().context("設定ディレクトリを取得できません")?;
    load_in(&config_dir)
}

/// `config_dir` 配下の `patch-catalog/` を cache にして読む。
pub(crate) fn load_in(config_dir: &Path) -> Result<PitchTrackingTable> {
    let source = CachedSource::new(
        "pitch-tracking",
        config_dir,
        PITCH_TRACKING_URL.to_string(),
        &format!("patch-catalog/{PITCH_TRACKING_FILE}"),
    );
    if let Err(error) = source.refresh(&Mutex::new(()), validate) {
        crate::logging::global_log_sink(&format!(
            "pitch-tracking: event=refresh-failed error=\"{error:#}\""
        ));
    }
    PitchTrackingTable::parse(source.read_cached()?.as_bytes()).map_err(anyhow::Error::msg)
}

fn validate(bytes: &[u8]) -> Result<()> {
    PitchTrackingTable::parse(bytes)
        .map(drop)
        .map_err(anyhow::Error::msg)
}

#[cfg(test)]
mod tests;
