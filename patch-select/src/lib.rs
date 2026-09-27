//! 音色の一覧（catalog）と、音色を選びながら試聴する selector。
//!
//! selector は「何を鳴らしてほしいか」を action で返すだけで、送信はしない。
//! 鳴らすものを MML・chord から作る変換と、送信する sender は selector を持つ側の crate にある。
//! そのため、ここにあるのは試聴で運ぶ型（[`LinePerformance`]・[`CursorNotes`] 等）だけ。

mod audition_select;
pub mod auto_reverb;
mod direct_select;
pub mod line_program;
pub mod notes;
mod patch_audition;
mod patch_catalog;
mod patch_select;
pub mod play_settings;
pub mod ui;

pub use audition_select::{
    AuditionMoment, PatchAuditionContext, PatchAuditionSelect, PatchCatalogNotice,
    PatchSelectOutcome,
};
pub use direct_select::{DirectPatchSelect, DirectPatchSelectRequest, DirectSelectOutcome};
pub use line_program::{FilterSettings, LinePerformance, LineProgram, LineStatus};
pub use notes::{CursorNotes, NoteRequest};
pub use patch_audition::{PatchAudition, PatchAuditionAction, PatchChange};
pub use patch_catalog::{
    host_patch_catalog, sort_for_selector, HostPatchCatalog, PatchCatalogEntry,
    PatchCatalogSnapshot,
};
pub use patch_select::{
    filter_candidates, prepare_user_presets, AutoReverbHost, AutoReverbKey, AutoReverbPanel,
    AutoReverbStatus, FilterGroup, FilterPreset, PatchSelect, PatchSelectAction,
    PatchSelectRequest, PreparedPresets, PAGE_STEP,
};
pub use play_settings::{is_play_settings_trigger, PlaySettings, PlaySettingsSelect};

pub(crate) const NOTE_ON: u8 = 0x90;

type LogSink = fn(&str);
static LOG_SINK: std::sync::OnceLock<LogSink> = std::sync::OnceLock::new();

/// app 起動時に、グローバルログ（`log/log.txt`）への書き込み関数を注入する。
/// 未注入の場合、この crate のログは黙って捨てられる。
///
/// 直接ファイルへ書かないのは、書き先が実ユーザーの `log/log.txt` 固定で、
/// 他 crate のテストからこの crate を通したときもそこへ追記してしまうため。
pub fn set_log_sink(log: LogSink) {
    let _ = LOG_SINK.set(log);
}

/// selector の調査ログ。1 行 1 事象で、キーと値を空白区切りで並べる。
pub(crate) fn log_line(message: String) {
    if let Some(sink) = LOG_SINK.get() {
        sink(&format!("patch-select: {message}"));
    }
}

#[cfg(test)]
mod tests;
