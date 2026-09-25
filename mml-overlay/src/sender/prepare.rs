//! 鳴らす前の音色の用意と、その間の「読み込み中」表示。

use std::sync::Mutex;

use super::live_patch::LivePatch;
use super::sink::SoundSink;
use super::status::MmlOverlaySenderStatus;
use super::voice::Voice;

pub(super) fn prepare_if_needed(
    voice: &mut Voice,
    sink: &impl SoundSink,
    status: &Mutex<MmlOverlaySenderStatus>,
    instance_id: u8,
    patch: &LivePatch,
) -> bool {
    if voice.is_patch_ready(instance_id, patch) {
        return true;
    }
    prepare_with_status(status, patch, || voice.prepare(sink, instance_id, patch))
}

/// 行の演奏の音色を用意する。行は音色に応じて instance を移る（[`Voice::prepare_line`]）。
pub(super) fn prepare_line_if_needed(
    voice: &mut Voice,
    sink: &impl SoundSink,
    status: &Mutex<MmlOverlaySenderStatus>,
    patch: &LivePatch,
) -> bool {
    if voice.is_line_patch_ready(sink, patch) {
        // 用意済みでも、いまの instance と違えば切り替える（読み込みは起きない）。
        return voice.prepare_line(sink, patch).is_ok();
    }
    prepare_with_status(status, patch, || voice.prepare_line(sink, patch))
}

/// 読み込みの間だけ画面へ「読み込み中」を出し、結果を失敗理由の欄へ落とす。
fn prepare_with_status(
    status: &Mutex<MmlOverlaySenderStatus>,
    patch: &LivePatch,
    prepare: impl FnOnce() -> Result<(), String>,
) -> bool {
    {
        let mut status = status.lock().unwrap();
        status.loading = true;
        status.loading_patch = patch.patch().map(str::to_string);
        status.sounding.clear();
    }
    let result = prepare();
    let mut status = status.lock().unwrap();
    status.loading = false;
    status.loading_patch = None;
    match result {
        Ok(()) => {
            status.prepare_error = None;
            true
        }
        Err(error) => {
            status.prepare_error = Some(error);
            status.prepare_error_command_id = status.command_id;
            false
        }
    }
}
