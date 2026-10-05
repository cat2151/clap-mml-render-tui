//! Drum matrix と、試聴しない kit 固定 selector の接続。

use cmrt_drum_sequencer::DrumSequencerScreen;
use cmrt_patch_select::{
    host_patch_catalog, ui::PatchSelectDrawOptions, HostPatchCatalog, PatchCatalogSnapshot,
    PatchSelect, PatchSelectAction, PatchSelectRequest,
};
use cmrt_tui_core::{patch_load::PatchLoadState, status::base_style, ui::centered_rect};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::TuiApp;

/// S05 の入力配送は、Continue / KitChanged のキーを背面へ渡さない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) enum DrumSequencerAction {
    Unhandled,
    Continue,
    /// 確定した kit へ差し替えた。host は古い preview を止める。
    KitChanged,
}

enum DrumKitSelector<'a> {
    Waiting,
    Notice(String),
    Select(Box<PatchSelect<'a>>),
}

/// セッション保存には含めない。画面往復時も同じ状態を使う。
#[derive(Default)]
pub(in crate::tui) struct DrumSequencerState<'a> {
    pub(in crate::tui) screen: DrumSequencerScreen,
    pub(in crate::tui) preview_command: Option<u64>,
    pub(in crate::tui) preview_error: Option<String>,
    selector: Option<DrumKitSelector<'a>>,
}

impl DrumSequencerState<'_> {
    pub(in crate::tui) fn selector_open(&self) -> bool {
        self.selector.is_some()
    }

    pub(in crate::tui) fn uses_textarea_cursor(&self) -> bool {
        matches!(&self.selector, Some(DrumKitSelector::Select(select)) if select.filter_editing())
    }

    pub(in crate::tui) fn close_selector(&mut self) {
        self.selector = None;
    }

    /// Loading 中の予約だけを差し替える。固定条件は Ready 後の open にも適用する。
    pub(in crate::tui) fn sync_catalog(&mut self, state: &PatchLoadState) {
        if matches!(self.selector, Some(DrumKitSelector::Waiting)) {
            self.open_selector(state);
        }
    }

    fn open_selector(&mut self, state: &PatchLoadState) {
        let HostPatchCatalog {
            catalog,
            patch_role_index,
            load_measurements,
        } = host_patch_catalog(state);
        self.selector = Some(match catalog {
            PatchCatalogSnapshot::Loading => DrumKitSelector::Waiting,
            PatchCatalogSnapshot::Error(reason) => DrumKitSelector::Notice(reason),
            PatchCatalogSnapshot::Ready(patches) => {
                let select = PatchSelect::open(PatchSelectRequest {
                    patches,
                    current: self.screen.kit_name().map(str::to_string),
                    role_index: patch_role_index,
                    drum_kit_only: true,
                    catalog_notes: match state {
                        PatchLoadState::Ready(snapshot) => snapshot.catalog_notes().to_vec(),
                        _ => Vec::new(),
                    },
                    load_measurements,
                    ..Default::default()
                });
                match select {
                    Some(select) if select.filtered_len() > 0 => DrumKitSelector::Select(Box::new(select)),
                    _ => DrumKitSelector::Notice(
                        "選択できる Drum kit がありません。cmrt build-patch-catalog-cache で catalog を再生成してください。".to_string(),
                    ),
                }
            }
        });
    }

    /// selector が最優先。確定 Enter / 取消 Esc を matrix へ伝播させない。
    pub(in crate::tui) fn handle_key(
        &mut self,
        key: KeyEvent,
        state: &PatchLoadState,
    ) -> DrumSequencerAction {
        self.sync_catalog(state);
        if let Some(selector) = self.selector.as_mut() {
            if key.kind == KeyEventKind::Release {
                return DrumSequencerAction::Continue;
            }
            match selector {
                DrumKitSelector::Waiting | DrumKitSelector::Notice(_) => {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Esc {
                        self.close_selector();
                    }
                }
                DrumKitSelector::Select(select) => match select.handle_key(key) {
                    PatchSelectAction::Confirm(patch) => {
                        let notes = select
                            .load_measurement(&patch)
                            .and_then(|measurement| measurement.drum_kit_notes.clone());
                        self.screen.set_kit(patch, notes);
                        self.close_selector();
                        return DrumSequencerAction::KitChanged;
                    }
                    PatchSelectAction::Cancel => self.close_selector(),
                    // 固定モードは試聴・設定・保存 action を生成しない。
                    _ => {}
                },
            }
            return DrumSequencerAction::Continue;
        }
        if key.modifiers == KeyModifiers::NONE && key.code == KeyCode::Char('t') {
            if key.kind == KeyEventKind::Press {
                self.open_selector(state);
            }
            return DrumSequencerAction::Continue;
        }
        if self.screen.handle_key_event(key) {
            DrumSequencerAction::Continue
        } else {
            DrumSequencerAction::Unhandled
        }
    }

    /// matrix の後に重ねる。演奏設定や auto reverb は描かない。
    pub(in crate::tui) fn draw_selector(&self, frame: &mut Frame<'_>) {
        let Some(selector) = &self.selector else {
            return;
        };
        let area = centered_rect(94, 78, frame.area());
        match selector {
            DrumKitSelector::Select(select) => cmrt_patch_select::ui::draw_patch_select_in(
                select,
                frame,
                area,
                &PatchSelectDrawOptions {
                    show_play_settings_hint: false,
                    ..Default::default()
                },
            ),
            notice => {
                let message = match notice {
                    DrumKitSelector::Waiting => {
                        "patch catalog を読み込み中です。完了後に Drum kit 選択を開きます。"
                    }
                    DrumKitSelector::Notice(reason) => reason.as_str(),
                    DrumKitSelector::Select(_) => unreachable!(),
                };
                frame.render_widget(Clear, area);
                frame.render_widget(
                    Paragraph::new(message)
                        .style(base_style())
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(" Drum kit 選択  Esc:取消 "),
                        )
                        .wrap(Wrap { trim: false }),
                    area,
                );
            }
        }
    }
}

impl TuiApp<'_> {
    pub(in crate::tui) fn handle_drum_sequencer_key_event(
        &mut self,
        key: KeyEvent,
    ) -> DrumSequencerAction {
        let state = self.patch_load_state.lock().unwrap().clone();
        let action = self.drum_sequencer.handle_key(key, &state);
        if action == DrumSequencerAction::KitChanged {
            self.stop_drum_sequencer_preview();
            self.drum_sequencer.preview_error = None;
        }
        action
    }

    pub(in crate::tui) fn sync_drum_sequencer_catalog(&mut self) {
        if self.drum_sequencer.selector_open() {
            let state = self.patch_load_state.lock().unwrap().clone();
            self.drum_sequencer.sync_catalog(&state);
        }
    }
}

#[cfg(test)]
mod tests;
