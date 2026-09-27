//! track の init セルの音色名を差し替える、DAW で 1 つだけの実装。
//!
//! 音色が変わる経路は 2 つある（`r` のランダム音色と、MML overlay での音色確定）。
//! 書き換える対象（init セルの JSON の音色名だけ）も、その後始末（依存セルの
//! 無効化・再レンダリングの一括投入・保存・hot reload）も同一なので、片方だけ直して
//! 食い違うことがないようここへ寄せてある。

use super::super::{playback_util::effective_measure_count, DawApp, FIRST_PLAYABLE_TRACK};
use super::normal::format_patch_hot_reload_log;

/// init 列。音色 JSON はここにだけ入る。
const INIT_MEASURE: usize = 0;

/// 音色を差し替えたのは誰か。ログの綴りをここ 1 か所に閉じる。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PatchUpdateReason {
    /// `r`（ランダム音色）。
    RandomPatch,
    /// MML overlay の `Ctrl+T` で音色を確定した。
    MmlOverlay,
}

impl PatchUpdateReason {
    /// 再レンダリング一括投入のログに出す理由。
    fn rerender_reason(self) -> &'static str {
        match self {
            Self::RandomPatch => "random patch update",
            Self::MmlOverlay => "mml overlay patch update",
        }
    }

    /// hot reload のログに出す語。
    fn hot_reload_label(self) -> &'static str {
        match self {
            Self::RandomPatch => "random patch",
            Self::MmlOverlay => "mml overlay patch",
        }
    }
}

impl DawApp {
    /// track の init セルの音色名だけを差し替え、キャッシュ・保存・hot reload まで面倒をみる。
    ///
    /// 既存の JSON はそのまま活かして音色名だけを置き換えるので、patch filter query の
    /// ような付随メタデータは壊れない。
    pub(crate) fn apply_patch_name_to_track_init(
        &mut self,
        track: usize,
        patch_name: &str,
        patch_filter_query: Option<&str>,
        reason: PatchUpdateReason,
    ) {
        if track < FIRST_PLAYABLE_TRACK || track >= self.editor.tracks {
            return;
        }
        let next_init = Self::replace_patch_name_in_mml(
            &self.editor.data[track][INIT_MEASURE],
            patch_name,
            patch_filter_query,
        );
        self.apply_track_init_cell(track, next_init, reason);
    }

    /// 音色名の差し替えと同時に、init セルの effect chain を `chain` にする。
    /// 再レンダリングは 1 回で済む。init セルが変わらなければ何もしない。
    pub(crate) fn apply_patch_name_and_effect_chain_to_track_init(
        &mut self,
        track: usize,
        patch_name: &str,
        patch_filter_query: Option<&str>,
        chain: &[serde_json::Value],
        reason: PatchUpdateReason,
    ) {
        if track < FIRST_PLAYABLE_TRACK || track >= self.editor.tracks {
            return;
        }
        let current_init = &self.editor.data[track][INIT_MEASURE];
        let next_init = crate::mml::effect_chain::init_cell_with_effect_chain(
            &Self::replace_patch_name_in_mml(current_init, patch_name, patch_filter_query),
            chain,
        );
        if next_init == *current_init {
            return;
        }
        self.apply_track_init_cell(track, next_init, reason);
    }

    fn apply_track_init_cell(
        &mut self,
        track: usize,
        next_init: String,
        reason: PatchUpdateReason,
    ) {
        let affected_measures: Vec<usize> = (1..=self.editor.measures)
            .filter(|&measure| crate::mml::cell_has_content(&self.editor.data, track, measure))
            .collect();
        self.editor.data[track][INIT_MEASURE] = next_init;
        self.invalidate_cell(track, INIT_MEASURE);
        self.invalidate_dependent_cells(track, INIT_MEASURE);
        self.start_track_rerender_batch(track, &affected_measures, reason.rerender_reason());
        self.save();

        // hot reload: 次の再生ループから新しい音色を反映する。
        // ロックを最小限に保つため、組み立ては先に済ませる。
        let new_mmls = self.build_measure_mmls();
        let new_samples = self.measure_duration_samples();
        let old_effective_count = {
            let old_mmls = self.playback.measure_mmls.lock().unwrap();
            effective_measure_count(&old_mmls)
        };
        let new_effective_count = effective_measure_count(&new_mmls);
        let old_samples = *self.playback.measure_samples.lock().unwrap();
        let displayed_measure_index = self
            .playback
            .position
            .lock()
            .unwrap()
            .as_ref()
            .map(|position| position.measure_index);
        self.append_log_line(format_patch_hot_reload_log(
            reason.hot_reload_label(),
            track,
            displayed_measure_index,
            old_effective_count,
            new_effective_count,
            old_samples,
            new_samples,
        ));
        self.sync_playback_mml_state();
    }
}
