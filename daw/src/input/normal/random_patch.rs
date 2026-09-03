//! `r`（ランダム音色）で、track の init セルの音色を選び直す。
//!
//! `r` キーと HTTP の random-patch コマンドが同じ入口を通る。

use super::super::super::{DawApp, FIRST_PLAYABLE_TRACK};
use super::super::track_patch::PatchUpdateReason;

impl DawApp {
    /// Applies the same random-patch update as pressing `r` on the target track.
    ///
    /// `Ok(false)` means no candidate patch was available, so the operation is a
    /// no-op. This matches the existing `r` key behavior, and HTTP callers also
    /// currently treat that case as a successful no-op.
    ///
    /// 抽選は **いま鳴っている音色と同じ用途（role）へ寄せる**。kick の track で
    /// pad が出てくると曲の作りが壊れるため。drum は role の中の部位
    /// （kick / snare / hat / perc）まで揃える。
    /// 用途を絞れるのは、track の init セルに書かれた音色を catalog が知っているときだけで、
    /// 知らない音色（分類できない・catalog 未読込）は従来どおり全体から抽選する。
    /// filter が書かれた track では、そちらを明示指定として優先する。
    pub(crate) fn apply_random_patch_to_track(&mut self, track: usize) -> Result<bool, String> {
        if track < FIRST_PLAYABLE_TRACK {
            return Err("ランダム音色は演奏トラックでのみ使用できます".to_string());
        }
        if track >= self.editor.tracks {
            return Err(format!(
                "track は {}..={} の範囲で指定してください",
                FIRST_PLAYABLE_TRACK,
                self.editor.tracks.saturating_sub(1)
            ));
        }
        let patch_filter_query = self.track_patch_filter_query(track);
        let same_role_patch = match (patch_filter_query.as_deref(), self.track_patch_name(track)) {
            (None, Some(current)) => self.pick_random_patch_name_for_same_role_as(&current),
            _ => None,
        };
        let patch = match same_role_patch {
            Some(patch) => patch,
            None => {
                let Some(patch) =
                    self.pick_random_patch_name_with_query(patch_filter_query.as_deref())
                else {
                    return Ok(false);
                };
                patch
            }
        };
        self.apply_patch_name_to_track_init(
            track,
            &patch,
            patch_filter_query.as_deref(),
            PatchUpdateReason::RandomPatch,
        );

        Ok(true)
    }
}
