//! MML 入力オーバーレイと共有ランタイムの接続。
//!
//! オーバーレイ自体は `cmrt-mml-overlay` crate に閉じている。ここが持つのは
//! 「どの画面から開いてよいか」「開くときに何を止めるか」「鳴らす先はどこか」
//! 「開くときに何のスナップショットを渡すか」の 4 つだけ。

use std::time::Instant;

use cmrt_patches::PatchRole;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::mml_overlay::{
    host_patch_catalog, is_mml_overlay_trigger, ChordChartPreviewContext, HostPatchCatalog,
    MmlOverlayAction, MmlOverlayContext, MmlOverlayInputMode, MmlOverlaySyntax, PatchChange,
    SingleLineFlow,
};
use super::{MmlOverlayOwner, PatchLoadState, TuiApp};

impl TuiApp<'_> {
    /// Ctrl+P、または Chord Chart の `t` / `Shift+T` ならオーバーレイを開く。
    /// 開いたら true。
    ///
    /// 開ける条件は画面切替メニューと同じにしてある。どちらも「いまの画面が
    /// モーダルな入力中でないこと」を求めるため。
    pub(in crate::tui) fn try_open_mml_overlay(&mut self, key: KeyEvent) -> bool {
        if self.active_screen == crate::screen_switch::PrimaryScreen::ChordChart {
            if let Some(role) = chord_chart_patch_selector_role(key) {
                if !self.can_open_screen_switch_menu() {
                    return false;
                }
                self.open_chord_chart_patch_selector(role);
                return true;
            }
        }
        if !is_mml_overlay_trigger(key) || !self.can_open_screen_switch_menu() {
            return false;
        }
        // オーバーレイは keyboard 画面と同じ音源インスタンスを借りるので、
        // 先にいまの画面の演奏を止めて明け渡してもらう。
        let context = self.mml_overlay_context();
        self.open_owned_mml_overlay(MmlOverlayOwner::Global, context);
        true
    }

    /// Chord Chart の `i` が返した stable id の section を、Modal 1 行 overlay で開く。
    /// action 発生後に section が消えていても no-op にする。
    pub(in crate::tui) fn open_chord_chart_degrees_overlay(
        &mut self,
        section_id: super::chord_chart::SectionId,
    ) {
        let Some(initial_text) = self
            .chord_chart
            .song
            .section(section_id)
            .map(|section| section.degrees.clone())
        else {
            return;
        };
        let key_token =
            super::chord_chart_glue::key_token(&self.chord_chart.song.prefix).map(str::to_owned);
        let mut context = self.mml_overlay_context();
        context.input_mode = MmlOverlayInputMode::SingleLine;
        context.single_line_flow = SingleLineFlow::Modal;
        context.initial_text = initial_text;
        context.syntax = MmlOverlaySyntax::ChordChart(ChordChartPreviewContext { key_token });
        context.patch_select_initial_role = Some(cmrt_patches::PatchRole::Chord);
        self.open_owned_mml_overlay(MmlOverlayOwner::ChordChart { section_id }, context);
    }

    /// Chord Chart の現在行を試聴文脈に載せ、指定 role の selector を直接開く。
    fn open_chord_chart_patch_selector(&mut self, role: PatchRole) {
        let initial_text = self
            .chord_chart
            .selected_preview_section()
            .map_or_else(String::new, |section| section.degrees.clone());
        let key_token =
            super::chord_chart_glue::key_token(&self.chord_chart.song.prefix).map(str::to_owned);
        let mut context = self.mml_overlay_context();
        context.input_mode = MmlOverlayInputMode::SingleLine;
        context.single_line_flow = SingleLineFlow::Modal;
        context.initial_text = initial_text;
        context.syntax = MmlOverlaySyntax::ChordChart(ChordChartPreviewContext { key_token });
        context.patch_select_initial_role = Some(role);
        self.open_owned_mml_overlay(MmlOverlayOwner::ChordChartPatch { role }, context);
        self.mml_overlay.request_patch_select();
    }

    /// owner の canonical patch を widget へ載せて、共有 overlay と sender を開く。
    fn open_owned_mml_overlay(&mut self, owner: MmlOverlayOwner, context: MmlOverlayContext) {
        self.stop_active_screen_playback();
        let patch = match owner {
            MmlOverlayOwner::Global => self.mml_overlay_patch.clone(),
            MmlOverlayOwner::ChordChart { .. } => self.chord_chart_patch.clone(),
            MmlOverlayOwner::ChordChartPatch {
                role: PatchRole::Chord,
            } => self.chord_chart_patch.clone(),
            MmlOverlayOwner::ChordChartPatch {
                role: PatchRole::Bass,
            } => self.chord_chart_bass_patch.clone(),
            MmlOverlayOwner::ChordChartPatch { .. } => None,
        };
        self.mml_overlay.set_restored_patch(patch);
        self.mml_overlay_owner = Some(owner);
        self.mml_overlay.open(context);
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.prepare(self.mml_overlay.patch());
            self.mml_overlay.expect_sender_command(command_id);
        }
    }

    /// 音色一覧の状態とフレーズ履歴を、開くたびに最新のスナップショットで渡す。
    /// 音色一覧が Loading なら、完了後に [`Self::pump_mml_overlay`] が差し替える。
    fn mml_overlay_context(&self) -> MmlOverlayContext {
        let HostPatchCatalog {
            catalog: patch_catalog,
            patch_role_index,
            load_measurements,
        } = self.mml_overlay_patch_catalog_snapshot();
        let (history, favorites) = self.notepad.phrase_history();
        let catalog_notes = match &*self.patch_load_state.lock().unwrap() {
            PatchLoadState::Ready(snapshot) if !snapshot.catalog_notes().is_empty() => {
                snapshot.catalog_notes().to_vec()
            }
            PatchLoadState::Ready(_) => self.catalog_notes.clone(),
            PatchLoadState::Loading | PatchLoadState::Err(_) => Vec::new(),
        };
        MmlOverlayContext {
            // app からの Ctrl+P は従来どおり複数行・空の入力欄で開く。
            // 1 行モードは DAW が明示的に指定したときだけ。
            input_mode: MmlOverlayInputMode::MultiLine,
            single_line_flow: Default::default(),
            initial_text: String::new(),
            syntax: Default::default(),
            patch_catalog,
            patch_role_index,
            patch_select_initial_role: None,
            load_measurements,
            history: history.to_vec(),
            favorites: favorites.to_vec(),
            patch_filter_presets: crate::history::load_mml_patch_filter_presets(),
            // notepad / keyboard / grid には chord 行が無い。移送先が無いので
            // chord のヒントも確認ダイアログも出さない。
            chord_row_transfer: false,
            catalog_notes,
        }
    }

    /// 一覧・Role 索引・load 計測は DAW と同じ 1 実装（`cmrt_mml_overlay::host_patch_catalog`）で
    /// 作る。`Loading` / `Err` のときに何を渡すかが画面ごとに食い違わないようにするため。
    fn mml_overlay_patch_catalog_snapshot(&self) -> HostPatchCatalog {
        host_patch_catalog(&self.patch_load_state.lock().unwrap())
    }

    #[cfg(test)]
    pub(in crate::tui) fn loaded_patch_pairs(&self) -> Vec<(String, String)> {
        match &*self.patch_load_state.lock().unwrap() {
            PatchLoadState::Ready(snapshot) => snapshot.pairs().to_vec(),
            PatchLoadState::Loading | PatchLoadState::Err(_) => Vec::new(),
        }
    }

    /// オーバーレイが開いている間、キーはすべてオーバーレイが取る。
    pub(in crate::tui) fn handle_mml_overlay_key_event(&mut self, key: KeyEvent) {
        // loader 完了と Ctrl+T が同じ frame に来ても、古い Loading を見せない。
        self.sync_mml_overlay_patch_catalog();
        let close_after_selector = matches!(
            self.mml_overlay_owner,
            Some(MmlOverlayOwner::ChordChartPatch { .. })
        ) && self.mml_overlay.is_patch_select_open();
        let action = self.mml_overlay.handle_key(key, Instant::now());
        self.remember_owned_mml_overlay_patch();
        self.apply_mml_overlay_action(action);
        // Chord Chart の直接 selector は、確定 / 取消後に空の MML editor を残さない。
        if close_after_selector
            && self.mml_overlay.is_open()
            && !self.mml_overlay.is_patch_select_open()
        {
            self.save_history_state();
            self.finish_owned_mml_overlay();
        }
    }

    /// worker が実際に到達した一覧・loading・発音状態を表示へ反映する。毎フレーム呼ぶ。
    pub(in crate::tui) fn pump_mml_overlay(&mut self) {
        self.sync_mml_overlay_patch_catalog();
        if let Some(sender) = &self.mml_overlay_sender {
            self.mml_overlay.sync_sender_status(&sender.status());
        }
    }

    fn sync_mml_overlay_patch_catalog(&mut self) {
        if !self.mml_overlay.is_waiting_for_patch_catalog() {
            return;
        }
        let HostPatchCatalog {
            catalog,
            patch_role_index,
            load_measurements,
        } = self.mml_overlay_patch_catalog_snapshot();
        self.mml_overlay
            .sync_patch_catalog(catalog, patch_role_index, load_measurements);
    }

    /// オーバーレイの求めを sender へ流す。
    ///
    /// note off はここには出てこない。「鳴っているものを止める」は sender 側が
    /// 1 か所で持っていて、音を鳴らすコマンドはどれも停止込みの意味になっている。
    fn apply_mml_overlay_action(&mut self, action: MmlOverlayAction) {
        let action = match action {
            MmlOverlayAction::SavePatchFilterPresets { presets, preview } => {
                if let Err(error) = crate::history::save_mml_patch_filter_presets(&presets) {
                    crate::logging::global_log_sink(&format!(
                        "mml-overlay: action=patch-filter-preset event=save result=error detail={error:?}"
                    ));
                } else if let PatchLoadState::Ready(snapshot) =
                    &mut *self.patch_load_state.lock().unwrap()
                {
                    std::sync::Arc::make_mut(snapshot).rebuild_patch_roles(&presets);
                }
                if let (Some(sender), Some((patch, notes))) = (&self.mml_overlay_sender, preview) {
                    let command_id = match notes {
                        Some(notes) => {
                            sender.send(Some(patch.as_str()), notes.messages, notes.duration)
                        }
                        None => sender.prepare(Some(patch.as_str())),
                    };
                    self.mml_overlay.expect_sender_command(command_id);
                }
                return;
            }
            action => action,
        };
        // 閉じるときだけ sender の外へ用がある（音源を借りていた画面へ返す）ので、
        // sender を借りる前に片づける。
        match action {
            MmlOverlayAction::Close => {
                self.finish_owned_mml_overlay();
                return;
            }
            MmlOverlayAction::Commit {
                ref line,
                close: true,
            } => {
                self.commit_owned_mml_overlay_line(line);
                self.finish_owned_mml_overlay();
                return;
            }
            _ => {}
        }
        let Some(sender) = &self.mml_overlay_sender else {
            return;
        };
        let command_id = match action {
            // Commit は 1 行モードでしか返らない。app 側は複数行モードでしか
            // 開かないのでここへは来ない（来ても sender へ用は無い）。
            // TransferToChordRow も同じで、chord 行を持つのは DAW 画面だけ
            // （`MmlOverlayContext::chord_row_transfer` を立てていない）。
            MmlOverlayAction::Continue
            | MmlOverlayAction::Close
            | MmlOverlayAction::Commit { .. }
            | MmlOverlayAction::TransferToChordRow { .. }
            | MmlOverlayAction::SavePatchFilterPresets { .. } => None,
            MmlOverlayAction::Send(notes) => {
                let id = sender.send(self.mml_overlay.patch(), notes.messages, notes.duration);
                Some(id)
            }
            MmlOverlayAction::SetPatch { patch, notes } => Some(match notes {
                Some(notes) => sender.send(patch.as_deref(), notes.messages, notes.duration),
                None => sender.prepare(patch.as_deref()),
            }),
            MmlOverlayAction::PlayLine { patch, program } => {
                let patch = match &patch {
                    PatchChange::Keep => self.mml_overlay.patch(),
                    PatchChange::Switch(patch) => patch.as_deref(),
                };
                Some(sender.play_line(patch, program))
            }
        };
        if let Some(command_id) = command_id {
            self.mml_overlay.expect_sender_command(command_id);
        }
    }

    /// Widget の patch は候補 preview では変わらず、selector の Enter 確定でだけ変わる。
    /// その変化を現在 owner の canonical patch へ反映する。
    fn remember_owned_mml_overlay_patch(&mut self) {
        let patch = self.mml_overlay.patch().map(str::to_owned);
        match self.mml_overlay_owner {
            Some(MmlOverlayOwner::Global) => self.mml_overlay_patch = patch,
            Some(MmlOverlayOwner::ChordChart { .. }) => self.chord_chart_patch = patch,
            Some(MmlOverlayOwner::ChordChartPatch {
                role: PatchRole::Chord,
            }) => self.chord_chart_patch = patch,
            Some(MmlOverlayOwner::ChordChartPatch {
                role: PatchRole::Bass,
            }) => self.chord_chart_bass_patch = patch,
            Some(MmlOverlayOwner::ChordChartPatch { .. }) => {}
            None => {}
        }
    }

    /// Modal 1 行編集の確定先を owner で振り分ける。
    fn commit_owned_mml_overlay_line(&mut self, line: &str) {
        let Some(MmlOverlayOwner::ChordChart { section_id }) = self.mml_overlay_owner else {
            return;
        };
        let line = line.trim();
        let Some(section) = self.chord_chart.song.section_mut(section_id) else {
            crate::logging::global_log_sink(&format!(
                "chord-chart: event=degrees-commit result=stale-section section_id={}",
                section_id.get()
            ));
            return;
        };
        if section.degrees == line {
            return;
        }
        section.degrees = line.to_owned();
        self.save_chord_chart();
        self.refresh_chord_chart_chord_ranges();
    }

    /// 閉じる経路を owner に関係なく 1 か所で片づける。
    fn finish_owned_mml_overlay(&mut self) {
        // 通常の Esc / Commit は widget 自身が既に閉じている。Chord Chart の直接
        // selector 確定 / 取消では selector だけが閉じるため、host 側でも確実に畳む。
        self.mml_overlay.dismiss();
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.stop();
            self.mml_overlay.expect_sender_command(command_id);
        }
        self.mml_overlay_owner = None;
        // widget に Chord Chart patch を残さない。次の Global open でも再設定するが、
        // 閉じている間に一時値を session 保存へ誤用されない状態に戻しておく。
        self.mml_overlay
            .set_restored_patch(self.mml_overlay_patch.clone());
        // 借りていた音源を返す。Chord Chart はここで自動 preview しない。
        self.resume_active_screen_playback();
    }
}

/// Chord Chart 専用の直接 selector キー。大文字は端末によって SHIFT の有無が揺れるため、
/// `Char('T')` 自体を Bass として扱い、CONTROL / ALT 付きは共有 shortcut へ譲る。
fn chord_chart_patch_selector_role(key: KeyEvent) -> Option<PatchRole> {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }
    match key.code {
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(PatchRole::Bass),
        KeyCode::Char('t') => Some(PatchRole::Chord),
        KeyCode::Char('T') => Some(PatchRole::Bass),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
