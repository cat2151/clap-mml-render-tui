//! Chord Chart の `x` で、Chord 音色に掛ける effect chain を選ぶ overlay。
//!
//! overlay は chain の写しを編集し、Enter で画面の chain へ書き戻す（Esc は捨てる）。
//! 試聴は `t` と同じく、カーソル section の進行を Chord role で、試聴したい chain を載せて鳴らす。
//! 開いている間は `t` の selector と同じ音源 instance を借りる。

use cmrt_effect_chain_select::{messages, AddKeyAction, ChainKeyAction, EffectChainEditor};
use crossterm::event::KeyEvent;
use serde_json::Value;

use super::super::mml_overlay::line_play::locally_auto_voiced_chord_chart_line_events;
use super::super::TuiApp;

/// 開いている overlay。`adding` は追加・差し替えの 3 pane を出しているか。
#[derive(Default)]
pub(in crate::tui) struct ChordChartEffectOverlay {
    pub(in crate::tui) editor: EffectChainEditor,
    pub(in crate::tui) adding: bool,
    /// overlay の中で出す、直近の操作ができなかった理由。
    pub(in crate::tui) error: Option<String>,
}

impl TuiApp<'_> {
    /// `x`: 確定済みの chain で overlay を開く。effect の catalog を持たない経路では開かず、
    /// 理由を Chord Chart の error 行に出す。
    pub(super) fn open_chord_chart_effect_chain(&mut self) {
        if self.effect_plugins.catalog().is_none() {
            self.chord_chart.error = Some(messages::NOT_AVAILABLE_ON_THIS_BACKEND.to_string());
            return;
        }
        // 音源は MML overlay と同じ instance を借りるので、いまの画面の演奏を止めて明け渡す。
        self.stop_active_screen_playback();
        self.chord_chart_effect_overlay = Some(ChordChartEffectOverlay {
            editor: EffectChainEditor::open(self.chord_chart_effect_chain_stages.clone()),
            ..ChordChartEffectOverlay::default()
        });
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.prepare(
                self.chord_chart_live_patch_with_stages(&self.chord_chart_effect_chain_stages),
            );
            self.mml_overlay.expect_sender_command(command_id);
        }
    }

    /// 開いている間、キーはすべて overlay が取る。
    pub(in crate::tui) fn handle_chord_chart_effect_chain_key_event(&mut self, key: KeyEvent) {
        let Some(mut overlay) = self.chord_chart_effect_overlay.take() else {
            return;
        };
        overlay.error = None;
        let catalog = self.effect_plugins.catalog();
        let preview = if overlay.adding {
            match overlay.editor.handle_add_key(catalog, key) {
                AddKeyAction::None | AddKeyAction::Help => None,
                AddKeyAction::Back | AddKeyAction::Committed => {
                    overlay.adding = false;
                    None
                }
                AddKeyAction::Preview { .. } => {
                    let cursor = overlay.editor.add.list_cursor;
                    overlay.editor.candidate_chain(catalog, cursor, false)
                }
                AddKeyAction::PreviewBypassed => {
                    let cursor = overlay.editor.add.list_cursor;
                    overlay.editor.candidate_chain(catalog, cursor, true)
                }
            }
        } else {
            match overlay.editor.handle_chain_key(key) {
                ChainKeyAction::None | ChainKeyAction::Help => None,
                ChainKeyAction::Preview => Some(overlay.editor.chain.clone()),
                ChainKeyAction::OpenAdd { replace_target } => match catalog {
                    Some(catalog) if !catalog.presets().is_empty() => {
                        overlay.editor.open_add(catalog, replace_target);
                        overlay.adding = true;
                        let cursor = overlay.editor.add.list_cursor;
                        overlay.editor.candidate_chain(Some(catalog), cursor, false)
                    }
                    _ => {
                        overlay.error = Some(messages::NO_PRESETS.to_string());
                        None
                    }
                },
                ChainKeyAction::Commit => {
                    self.chord_chart_effect_chain_stages = overlay.editor.chain;
                    self.save_history_state();
                    self.close_chord_chart_effect_chain();
                    return;
                }
                ChainKeyAction::Close => {
                    self.close_chord_chart_effect_chain();
                    return;
                }
            }
        };
        self.chord_chart_effect_overlay = Some(overlay);
        if let Some(stages) = preview {
            self.preview_chord_chart_effect_chain(&stages);
        }
    }

    /// カーソル section の進行を、Chord 音色に `stages` を掛けて鳴らす。進行が無ければ音色の準備だけ。
    fn preview_chord_chart_effect_chain(&mut self, stages: &[Value]) {
        let Some(sender) = &self.mml_overlay_sender else {
            return;
        };
        let degrees = self
            .chord_chart
            .selected_preview_section()
            .map_or_else(String::new, |section| section.degrees.clone());
        let key_token = super::super::chord_chart_glue::key_token(&self.chord_chart.song.prefix);
        let (_, performance) =
            locally_auto_voiced_chord_chart_line_events(&degrees, key_token, None);
        let patch = self.chord_chart_live_patch_with_stages(stages);
        let command_id = if performance.is_silent() {
            sender.prepare(patch)
        } else {
            sender.play_line(patch, self.mml_overlay.play_settings().program(performance))
        };
        self.mml_overlay.expect_sender_command(command_id);
    }

    /// 借りていた音源を画面へ返す。Chord Chart はここで自動 preview しない。
    fn close_chord_chart_effect_chain(&mut self) {
        self.chord_chart_effect_overlay = None;
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.stop();
            self.mml_overlay.expect_sender_command(command_id);
        }
        self.resume_active_screen_playback();
    }
}

/// Chord Chart 専用の effect chain キー。CONTROL / ALT 付きは共有 shortcut へ譲る。
pub(super) fn is_chord_chart_effect_chain_trigger(key: KeyEvent) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};
    key.code == KeyCode::Char('x')
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

/// `x` が掛ける先。overlay の 1 行目に出す。
pub(in crate::tui) fn chord_chart_effect_chain_header(patch: Option<&str>) -> String {
    format!("Chord 音色: {}", patch.unwrap_or("(既定音色)"))
}
