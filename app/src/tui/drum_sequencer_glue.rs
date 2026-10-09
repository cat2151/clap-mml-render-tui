//! Drum matrix と、候補 kit を試聴する kit 固定 selector の接続。

use cmrt_drum_sequencer::{DrumSequencerScreen, KitResolution};
use cmrt_mml_overlay::StepHit;
use cmrt_patch_select::{
    host_patch_catalog, ui::PatchSelectDrawOptions, HostPatchCatalog, PatchCatalogSnapshot,
    PatchSelect, PatchSelectAction, PatchSelectRequest,
};
use cmrt_tui_core::{
    patch_load::{PatchLoadMeasurement, PatchLoadState},
    status::base_style,
    ui::{centered_rect, clear_overlay_area},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::{drum_sequencer_preview::LoopHorizon, TuiApp};

mod audition_status;
mod kit_help;
mod pattern_store;

pub(in crate::tui) use audition_status::KitAuditionView;
pub(in crate::tui) use pattern_store::load_kit_patterns;

/// S05 の入力配送は、Continue / KitChanged のキーを背面へ渡さない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) enum DrumSequencerAction {
    Unhandled,
    Continue,
    /// 確定した kit へ差し替えた。host は古い preview を止める。
    KitChanged,
}

/// selector の候補が変わった、または Space で再試聴を求められたときに host が行う操作。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::tui) enum KitAudition {
    /// 候補 kit の割当 note を昇順に 1 周鳴らす。
    Play { patch: String, notes: Vec<u8> },
    /// 前の試聴を止めるだけ。候補なし・割当未知 / 空・selector を閉じたとき。
    Stop,
}

enum DrumKitSelector<'a> {
    Waiting,
    Notice(String),
    Select(Box<PatchSelect<'a>>),
}

/// 画面往復時も同じ状態を使う。入力は編集のたびに kit の pattern ファイルへ、`screen` の kit・
/// pattern 番号・カーソルは終了時に history へ保存する。
#[derive(Default)]
pub(in crate::tui) struct DrumSequencerState<'a> {
    pub(in crate::tui) screen: DrumSequencerScreen,
    pub(in crate::tui) preview_command: Option<u64>,
    pub(in crate::tui) preview_error: Option<String>,
    /// pattern ファイルの読み書きの失敗。kit を替えて読み直すまで status に出す。
    pub(in crate::tui) storage_error: Option<String>,
    /// 繰り返し再生中なら、最後に sender へ渡した打点。`preview_command` がそのループ。
    pub(in crate::tui) loop_hits: Option<Vec<StepHit>>,
    /// 繰り返し再生の先読み。保存しない。
    pub(in crate::tui) loop_horizon: LoopHorizon,
    selector: Option<DrumKitSelector<'a>>,
    /// 直近に試聴操作（再生か停止）を出した候補。selector を開くたびに空へ戻す。
    auditioned: Option<String>,
    replay_audition: bool,
    /// selector の上の `?` help。selector を閉じると一緒に閉じる。
    kit_help_open: bool,
    /// 自動開始を使い切った（自動で始めたか Shift+P を押した）。selector を閉じると戻す。
    pub(in crate::tui) auto_play_spent: bool,
}

impl DrumSequencerState<'_> {
    /// 保存から戻した画面で始める。再生・selector・help は閉じた状態。
    pub(in crate::tui) fn restored(
        screen: DrumSequencerScreen,
        storage_error: Option<String>,
    ) -> Self {
        Self {
            screen,
            storage_error,
            ..Default::default()
        }
    }

    /// selector の読み込み待ちか、保存から戻した kit の照合待ちがある。
    pub(in crate::tui) fn waits_for_catalog(&self) -> bool {
        self.selector_open() || self.screen.kit_resolution() == Some(KitResolution::Waiting)
    }

    pub(in crate::tui) fn selector_open(&self) -> bool {
        self.selector.is_some()
    }

    /// selector か help が開いている間は、global shortcut より先に全キーを受け取る。
    pub(in crate::tui) fn captures_keys(&self) -> bool {
        self.selector_open() || self.screen.help_open()
    }

    pub(in crate::tui) fn uses_textarea_cursor(&self) -> bool {
        matches!(&self.selector, Some(DrumKitSelector::Select(select)) if select.filter_editing())
    }

    /// 画面を離れるときも通るので、次に画面へ戻ったときも自動で回り始める。
    pub(in crate::tui) fn close_selector(&mut self) {
        self.selector = None;
        self.kit_help_open = false;
        self.auto_play_spent = false;
    }

    /// 保存から戻した kit を照合し、Loading 中の selector 予約を差し替える。
    /// 固定条件は Ready 後の open にも適用する。
    pub(in crate::tui) fn sync_catalog(&mut self, state: &PatchLoadState) {
        self.resolve_restored_kit(state);
        if matches!(self.selector, Some(DrumKitSelector::Waiting)) {
            self.open_selector(state);
        }
    }

    /// kit 確定と同じく、catalog の note 一覧と構成音名を採用する。
    /// catalog に無い・読めないときは名前と入力を残して未解決にする。
    fn resolve_restored_kit(&mut self, state: &PatchLoadState) {
        if self.screen.kit_resolution() != Some(KitResolution::Waiting) {
            return;
        }
        let snapshot = match state {
            PatchLoadState::Loading => return,
            PatchLoadState::Err(_) => {
                self.screen.mark_kit_missing();
                return;
            }
            PatchLoadState::Ready(snapshot) => snapshot,
        };
        let Some(name) = self.screen.kit_name().map(str::to_string) else {
            return;
        };
        if !snapshot.pairs().iter().any(|(display, _)| *display == name) {
            self.screen.mark_kit_missing();
            return;
        }
        let (notes, names, one_shot) = kit_notes(snapshot.load_measurements().get(&name));
        self.screen.set_kit(name, notes, names, one_shot);
    }

    /// 候補が前回の試聴から変わったか、Space が押されたときだけ操作を返す。
    ///
    /// 重い kit は候補に移っただけでは鳴らさず、前の試聴を止めるだけにする。読み込みは
    /// 送信 worker を塞ぎ途中で止められないので、通り過ぎただけで後続の試聴が十数秒鳴らなくなる。
    pub(in crate::tui) fn take_kit_audition(&mut self) -> Option<KitAudition> {
        let replay = std::mem::take(&mut self.replay_audition);
        let (target, measurement) = match &self.selector {
            Some(DrumKitSelector::Select(select)) => match select.selected() {
                Some(patch) => (Some(patch.to_string()), select.load_measurement(patch)),
                None => (None, None),
            },
            _ => (None, None),
        };
        if target == self.auditioned && !(replay && target.is_some()) {
            return None;
        }
        let heavy = measurement.is_some_and(PatchLoadMeasurement::is_heavy_offline_load);
        let notes = measurement.and_then(|measurement| measurement.drum_kit_notes.clone());
        self.auditioned = target.clone();
        Some(match (target, notes) {
            (Some(patch), Some(notes)) if !notes.is_empty() && (replay || !heavy) => {
                KitAudition::Play { patch, notes }
            }
            _ => KitAudition::Stop,
        })
    }

    /// selector の候補 kit の割当 note が、試聴できない理由。試聴できるなら `None`。
    pub(in crate::tui) fn audition_unavailable_reason(&self) -> Option<&'static str> {
        let Some(DrumKitSelector::Select(select)) = &self.selector else {
            return None;
        };
        let patch = select.selected()?;
        match select
            .load_measurement(patch)
            .and_then(|measurement| measurement.drum_kit_notes.as_deref())
        {
            None => Some("試聴不可: 割当 note 不明（cmrt build-patch-catalog-cache で再生成）"),
            Some([]) => Some("試聴不可: 割当 note なし"),
            Some(_) => None,
        }
    }

    fn open_selector(&mut self, state: &PatchLoadState) {
        self.auditioned = None;
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

    /// selector が最優先。その上の help はさらに先。確定 Enter / 取消 Esc を matrix へ伝播させない。
    pub(in crate::tui) fn handle_key(
        &mut self,
        key: KeyEvent,
        state: &PatchLoadState,
    ) -> DrumSequencerAction {
        self.sync_catalog(state);
        if self.selector_open()
            && key.kind != KeyEventKind::Release
            && self.handle_kit_help_key(key)
        {
            return DrumSequencerAction::Continue;
        }
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
                        let (notes, names, one_shot) = kit_notes(select.load_measurement(&patch));
                        // 入力は kit ごと。確定した kit の保存済み pattern に差し替える。
                        let (patterns, error) = load_kit_patterns(&patch);
                        self.screen.set_kit(patch, notes, names, one_shot);
                        self.screen.set_patterns(patterns);
                        self.storage_error = error;
                        self.close_selector();
                        return DrumSequencerAction::KitChanged;
                    }
                    PatchSelectAction::Cancel => self.close_selector(),
                    PatchSelectAction::PlayLine(_) => self.replay_audition = true,
                    // 固定モードは候補移動の Preview・設定・保存 action を生成しない。
                    _ => {}
                },
            }
            return DrumSequencerAction::Continue;
        }
        if !self.screen.help_open()
            && key.modifiers == KeyModifiers::NONE
            && key.code == KeyCode::Char('t')
        {
            if key.kind == KeyEventKind::Press {
                self.screen.clear_count();
                self.open_selector(state);
            }
            return DrumSequencerAction::Continue;
        }
        if !self.screen.handle_key_event(key) {
            return DrumSequencerAction::Unhandled;
        }
        if let Err(error) = pattern_store::save_edited_pattern(&mut self.screen) {
            self.storage_error = Some(error);
        }
        DrumSequencerAction::Continue
    }

    /// matrix の後に重ねる。演奏設定や auto reverb は描かない。
    /// 試聴の状態は一覧の下、selector の枠内の最終行に出す（枠外は背面の matrix と混ざる）。
    pub(in crate::tui) fn draw_selector(&self, frame: &mut Frame<'_>, view: &KitAuditionView) {
        let Some(selector) = &self.selector else {
            return;
        };
        let area = centered_rect(94, 78, frame.area());
        match selector {
            DrumKitSelector::Select(select) => {
                let [list_area, status_area] =
                    Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);
                clear_overlay_area(frame, area);
                let marker = |patch: &str| view.marker(patch);
                cmrt_patch_select::ui::draw_patch_select_in(
                    select,
                    frame,
                    list_area,
                    &PatchSelectDrawOptions {
                        patch_marker: Some(&marker),
                        show_play_settings_hint: false,
                    },
                );
                frame.render_widget(
                    Paragraph::new(view.line.as_str()).style(base_style().patch(view.line_style())),
                    status_area,
                );
            }
            notice => {
                let message = match notice {
                    DrumKitSelector::Waiting => {
                        "patch catalog を読み込み中です。完了後に Drum kit 選択を開きます。"
                    }
                    DrumKitSelector::Notice(reason) => reason.as_str(),
                    DrumKitSelector::Select(_) => unreachable!(),
                };
                clear_overlay_area(frame, area);
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
        if self.kit_help_open() {
            kit_help::draw_overlay(frame);
        }
    }
}

/// [`DrumSequencerScreen::set_kit`] へ渡す、kit の割当 note・構成音名・one-shot の note。
type KitNotes = (Option<Vec<u8>>, Vec<(u8, String)>, Vec<u8>);

/// catalog の計測から作る。計測が無ければ不明・空。
fn kit_notes(measurement: Option<&PatchLoadMeasurement>) -> KitNotes {
    measurement
        .map(|measurement| {
            (
                measurement.drum_kit_notes.clone(),
                measurement.drum_kit_note_names.clone(),
                measurement.drum_kit_one_shot_notes.clone(),
            )
        })
        .unwrap_or_default()
}

impl TuiApp<'_> {
    pub(in crate::tui) fn handle_drum_sequencer_key_event(
        &mut self,
        key: KeyEvent,
    ) -> DrumSequencerAction {
        let state = self.patch_load_state.lock().unwrap().clone();
        let selector_was_open = self.drum_sequencer.selector_open();
        let action = self.drum_sequencer.handle_key(key, &state);
        let audition = self.drum_sequencer.take_kit_audition();
        let turned_on = self.drum_sequencer.screen.take_turned_on();
        if !selector_was_open && self.drum_sequencer.selector_open() {
            // kit 一覧を開いたら繰り返し再生を止める。候補の試聴はこの後に積む。
            self.stop_drum_sequencer_preview();
        }
        if action == DrumSequencerAction::KitChanged {
            self.stop_drum_sequencer_preview();
            self.drum_sequencer.preview_error = None;
        } else if let Some(audition) = audition {
            self.apply_kit_audition(audition);
        } else if action == DrumSequencerAction::Continue {
            self.sync_drum_sequencer_loop_hits();
            if let Some(cell) = turned_on {
                self.shoot_drum_sequencer_cell(cell);
            }
        }
        action
    }

    /// 保存した kit を catalog と照合する。読み込み待ちで予約した selector が開いたら、
    /// 最初の候補を試聴する。
    pub(in crate::tui) fn sync_drum_sequencer_catalog(&mut self) {
        if self.drum_sequencer.waits_for_catalog() {
            let state = self.patch_load_state.lock().unwrap().clone();
            self.drum_sequencer.sync_catalog(&state);
            if let Some(audition) = self.drum_sequencer.take_kit_audition() {
                self.apply_kit_audition(audition);
            }
        }
    }
}

#[cfg(test)]
mod tests;
