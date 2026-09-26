//! 音色 selector の試聴で、前の候補の余韻を消してから次の候補を鳴らす。
//!
//! 候補ごとに音色が違うので、前の候補の release や effect の余韻が残ると聴き比べられない。
//! 消し方は EFFECT CHAIN の試聴と同じ `fade_out_line`（0 に達した instance の音源と
//! effect chain を server が reset する）。全NoteOff では effect の余韻が消えない。

use cmrt_mml_overlay::MmlOverlayAction;

use super::super::DawApp;

/// 前の候補を絞る長さ。0 にすると段差でクリックする。
const PATCH_PREVIEW_FADE_OUT_MS: u32 = 50;

impl DawApp {
    /// selector が開いている間の発音だけを対象にする。打鍵の試聴は余韻ごと聴かせる。
    pub(super) fn fade_out_before_patch_preview(&self, action: &MmlOverlayAction) {
        let sounds = matches!(
            action,
            MmlOverlayAction::PlayLine { .. } | MmlOverlayAction::SetPatch { notes: Some(_), .. }
        );
        if !sounds || !self.mml_overlay.is_patch_select_open() {
            return;
        }
        let Some(sender) = &self.mml_overlay_sender else {
            return;
        };
        if let Err(error) = sender.fade_out_line(PATCH_PREVIEW_FADE_OUT_MS) {
            self.append_log_line(format!("patch-preview: fade-out error {error}"));
        }
    }
}

#[cfg(test)]
mod tests;
