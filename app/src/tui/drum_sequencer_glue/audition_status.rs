//! kit selector の枠内に出す試聴の状態行と、一覧の行頭の印。
//!
//! 読み込みは送信 worker を塞ぎ、途中で止められない。今の読み込みが自分の試聴か、
//! 前の候補の残りかを分けて出さないと、無音が「壊れた」に見える。

use std::time::Instant;

use cmrt_tui_core::theme::{MONOKAI_CYAN, MONOKAI_PINK};
use ratatui::{style::Style, text::Span};

use super::{DrumKitSelector, TuiApp};

/// 状態行と行頭の印を決める材料。描画ごとに作り直す。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::tui) struct KitAuditionView {
    pub(super) line: String,
    /// 読み込みの待ちを示す行か。色を変える。
    pub(super) waiting: bool,
    pub(super) loading_patch: Option<String>,
    pub(super) sounding_patch: Option<String>,
}

const LOADING_MARK: &str = " …";
const SOUNDING_MARK: &str = " ♪";

impl KitAuditionView {
    pub(super) fn line_style(&self) -> Style {
        if self.waiting {
            Style::default().fg(MONOKAI_PINK)
        } else {
            Style::default()
        }
    }

    pub(super) fn marker(&self, patch: &str) -> Span<'static> {
        if self.loading_patch.as_deref() == Some(patch) {
            Span::styled(LOADING_MARK, Style::default().fg(MONOKAI_PINK))
        } else if self.sounding_patch.as_deref() == Some(patch) {
            Span::styled(SOUNDING_MARK, Style::default().fg(MONOKAI_CYAN))
        } else {
            Span::raw("")
        }
    }
}

impl TuiApp<'_> {
    pub(in crate::tui) fn kit_audition_view(&self, now: Instant) -> KitAuditionView {
        let status = self
            .mml_overlay_sender
            .as_ref()
            .map(|sender| sender.status())
            .unwrap_or_default();
        let command = self.drum_sequencer.preview_command;
        let loading_patch = status
            .is_loading()
            .then(|| status.loading_patch().map(str::to_string))
            .flatten();
        let sounding_patch = status
            .line_playback()
            .filter(|playback| {
                Some(playback.command_id()) == command && playback.is_sounding_at(now)
            })
            .and(self.drum_sequencer.auditioned.clone());
        let (line, waiting) = self.kit_audition_line(
            status.is_loading() && Some(status.command_id()) == command,
            loading_patch.as_deref(),
            sounding_patch.is_some(),
            now,
        );
        KitAuditionView {
            line,
            waiting,
            loading_patch,
            sounding_patch,
        }
    }

    fn kit_audition_line(
        &self,
        loading_own: bool,
        loading_patch: Option<&str>,
        sounding: bool,
        now: Instant,
    ) -> (String, bool) {
        let state = &self.drum_sequencer;
        if let Some(error) = &state.preview_error {
            return (error.clone(), false);
        }
        if let Some(patch) = loading_patch {
            let elapsed = self
                .sound_startup_wait
                .as_ref()
                .map(|wait| format!(" {:.1}s", now.duration_since(wait.started_at).as_secs_f64()))
                .unwrap_or_default();
            let line = if loading_own {
                format!("読込中{elapsed} … {patch}")
            } else {
                format!("読込待ち{elapsed}: {patch} の読込が終わるまで鳴りません（中断できません）")
            };
            return (line, true);
        }
        if let Some(reason) = state.audition_unavailable_reason() {
            return (reason.to_string(), false);
        }
        if sounding {
            return (
                "試聴中: 低い note から 0.25 秒ごと  Space:もう一度".to_string(),
                false,
            );
        }
        let heavy_bytes = match &state.selector {
            Some(DrumKitSelector::Select(select)) => select
                .selected()
                .and_then(|patch| select.load_measurement(patch))
                .filter(|measurement| measurement.is_heavy_offline_load())
                .and_then(|measurement| measurement.sfz_sample_bytes),
            _ => None,
        };
        match heavy_bytes {
            Some(bytes) => (
                format!(
                    "重い kit ({}MB): 移っただけでは鳴らしません  Space:読み込んで試聴（数秒〜十数秒）",
                    bytes / 1_000_000
                ),
                false,
            ),
            None => ("Space:もう一度試聴".to_string(), false),
        }
    }
}
