use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_explorer::Input;

use super::super::{
    daily::daily_archive_root, messages::project as message, project::DEFAULT_PROJECT_FILE_NAME,
    DawApp, DawMode, DawPlayState, DawProjectFileAction, WorkspaceKind,
};

impl DawApp {
    pub(crate) fn accepts_project_file_key(&self, modifiers: KeyModifiers) -> bool {
        modifiers == KeyModifiers::NONE && self.workspace_kind == WorkspaceKind::Persistent
    }

    pub(crate) fn start_project_overlay(&mut self) {
        if self.workspace_kind != WorkspaceKind::Persistent {
            return;
        }
        self.overlays.project.action = None;
        self.overlays.project.error = None;
        self.overlays.project.backup_notice_path = None;
        self.mode = DawMode::Project;
    }

    fn start_project_path_input(&mut self, action: DawProjectFileAction) {
        debug_assert_eq!(action, DawProjectFileAction::SaveAs);
        let initial_path = self
            .overlays
            .project
            .current_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| DEFAULT_PROJECT_FILE_NAME.to_string());
        self.overlays.project.path_textarea =
            cmrt_tui_core::text_input::new_single_line_textarea(&initial_path);
        self.overlays.project.action = Some(action);
        self.overlays.project.error = None;
    }

    fn execute_project_file_action(&mut self, action: DawProjectFileAction) {
        debug_assert_eq!(action, DawProjectFileAction::SaveAs);
        let path_text =
            cmrt_tui_core::text_input::textarea_value(&self.overlays.project.path_textarea);
        match self.save_project_as(&path_text) {
            Ok(saved) => {
                self.append_log_line(format!("project saved: {}", saved.path.display()));
                if let Some(backup_path) = &saved.backup_path {
                    self.append_log_line(format!(
                        "project backup created: {}",
                        backup_path.display()
                    ));
                }
                self.overlays.project.current_path = Some(saved.path);
                self.overlays.project.backup_notice_path = saved.backup_path;
                self.overlays.project.action = None;
                self.overlays.project.error = None;
                if self.overlays.project.backup_notice_path.is_none() {
                    self.mode = DawMode::Normal;
                }
            }
            Err(error) => {
                self.overlays.project.error = Some(error.to_string());
            }
        }
    }

    fn execute_project_open(&mut self, path: &std::path::Path, action: DawProjectFileAction) {
        debug_assert!(action.is_open());
        match self.open_project(&path.to_string_lossy()) {
            Ok(path) => {
                if action == DawProjectFileAction::OpenDailyArchive {
                    self.append_log_line(format!(
                        "daily archive opened as copy: {}",
                        path.display()
                    ));
                    self.overlays.project.current_path = None;
                } else {
                    self.append_log_line(format!("project opened: {}", path.display()));
                    self.overlays.project.current_path = Some(path);
                }
                self.overlays.project.file_explorer = None;
                self.overlays.project.action = None;
                self.overlays.project.error = None;
                self.overlays.project.backup_notice_path = None;
                self.mode = DawMode::Normal;
            }
            Err(error) => self.overlays.project.error = Some(error.to_string()),
        }
    }

    fn stop_project_file_preview(&self) {
        if *self.playback.play_state.lock().unwrap() == DawPlayState::Preview {
            self.stop_play();
        }
    }

    fn preview_selected_project_file(&mut self, force: bool) {
        let Some(path) = self.overlays.project.selected_path() else {
            return;
        };
        if !force && self.overlays.project.previewed_path.as_ref() == Some(&path) {
            return;
        }

        self.stop_project_file_preview();
        self.overlays.project.previewed_path = Some(path.clone());
        self.overlays.project.preview_info = None;
        self.overlays.project.preview_error = None;
        if path.is_dir() {
            self.overlays.project.preview_info = Some(message::DIRECTORY_PREVIEW.to_string());
            return;
        }

        match self.inspect_project_for_preview(&path) {
            Ok(preview) => {
                let measure_label = preview
                    .measure_index
                    .map(message::preview_measure)
                    .unwrap_or_else(|| message::NO_PLAYABLE_MEASURE.to_string());
                self.overlays.project.preview_info = Some(message::preview_summary(
                    preview.tracks,
                    preview.measures,
                    &measure_label,
                ));
                let Some(measure_index) = preview.measure_index else {
                    return;
                };
                if *self.playback.play_state.lock().unwrap() == DawPlayState::Playing {
                    self.overlays.project.preview_error =
                        Some(message::PLAYBACK_ACTIVE_PREVIEW_SKIPPED.to_string());
                    return;
                }
                if self.try_start_preview_with_track_mmls_for_test(
                    measure_index,
                    Some(preview.track_mmls.clone()),
                ) {
                    return;
                }
                self.start_uncached_preview_with_snapshot(
                    measure_index,
                    preview.track_mmls,
                    preview.track_gains,
                    preview.measure_samples,
                );
            }
            Err(error) => self.overlays.project.preview_error = Some(error.to_string()),
        }
    }

    fn project_open_selection_changed(&mut self) {
        self.stop_project_file_preview();
        self.overlays.project.error = None;
        self.overlays.project.previewed_path = None;
        self.overlays.project.preview_info = None;
        self.overlays.project.preview_error = None;
        if self.overlays.project.auto_preview {
            self.preview_selected_project_file(false);
        }
    }

    fn handle_project_open_key_event(&mut self, key_event: KeyEvent) {
        if self.overlays.project.filter_active {
            let previous_path = self.overlays.project.selected_path();
            cmrt_tui_core::text_input::sync_single_line_textarea(
                &mut self.overlays.project.query_textarea,
                &self.overlays.project.query,
            );
            match key_event.code {
                KeyCode::Esc => {
                    self.overlays.project.filter_active = false;
                    self.overlays.project.query = self.overlays.project.query_before_input.clone();
                    self.overlays.project.query_textarea =
                        cmrt_tui_core::text_input::new_single_line_textarea(
                            &self.overlays.project.query,
                        );
                    if let Err(error) = self.overlays.project.apply_query() {
                        self.overlays.project.error = Some(error.to_string());
                    }
                }
                KeyCode::Enter => self.overlays.project.filter_active = false,
                _ => {
                    if cmrt_tui_core::text_input::apply_key_event_to_textarea(
                        &mut self.overlays.project.query_textarea,
                        key_event,
                    ) {
                        self.overlays.project.query = cmrt_tui_core::text_input::textarea_value(
                            &self.overlays.project.query_textarea,
                        );
                        if let Err(error) = self.overlays.project.apply_query() {
                            self.overlays.project.error = Some(error.to_string());
                        } else {
                            self.overlays.project.error = None;
                        }
                    }
                }
            }
            if self.overlays.project.selected_path() != previous_path {
                self.project_open_selection_changed();
            }
            return;
        }

        if key_event.code == KeyCode::Esc {
            self.stop_project_file_preview();
            self.overlays.project.file_explorer = None;
            self.overlays.project.action = None;
            self.overlays.project.error = None;
            self.overlays.project.previewed_path = None;
            self.overlays.project.preview_info = None;
            self.overlays.project.preview_error = None;
            return;
        }

        let previous_path = self.overlays.project.selected_path();
        let selected_path = self.overlays.project.selected_path();
        let selected_is_dir = selected_path.as_ref().is_some_and(|path| path.is_dir());
        match key_event.code {
            KeyCode::Char('/') if key_event.modifiers == KeyModifiers::NONE => {
                self.overlays.project.query_before_input = self.overlays.project.query.clone();
                self.overlays.project.filter_active = true;
            }
            KeyCode::Char('a') if key_event.modifiers == KeyModifiers::NONE => {
                self.overlays.project.auto_preview = !self.overlays.project.auto_preview;
                self.stop_project_file_preview();
                self.overlays.project.previewed_path = None;
                if self.overlays.project.auto_preview {
                    self.preview_selected_project_file(false);
                }
            }
            KeyCode::Char(' ') if key_event.modifiers == KeyModifiers::NONE => {
                self.preview_selected_project_file(true);
            }
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right if selected_is_dir => {
                if let Some(explorer) = self.overlays.project.file_explorer.as_mut() {
                    if let Err(error) = explorer.handle(Input::Right) {
                        self.overlays.project.error = Some(error.to_string());
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(path) = selected_path {
                    let action = self
                        .overlays
                        .project
                        .action
                        .expect("open selector has an action");
                    self.execute_project_open(&path, action);
                }
            }
            _ => {
                if let Some(explorer) = self.overlays.project.file_explorer.as_mut() {
                    if let Err(error) =
                        explorer.handle(Input::from(&crossterm::event::Event::Key(key_event)))
                    {
                        self.overlays.project.error = Some(error.to_string());
                    }
                }
            }
        }
        if self.mode == DawMode::Project
            && self
                .overlays
                .project
                .action
                .is_some_and(DawProjectFileAction::is_open)
            && self.overlays.project.selected_path() != previous_path
        {
            self.project_open_selection_changed();
        }
    }

    pub(crate) fn handle_project_key_event(&mut self, key_event: KeyEvent) {
        if let Some(action) = self.overlays.project.action {
            if action.is_open() {
                self.handle_project_open_key_event(key_event);
                return;
            }
            match key_event.code {
                KeyCode::Esc => {
                    self.overlays.project.action = None;
                    self.overlays.project.error = None;
                }
                KeyCode::Enter => self.execute_project_file_action(action),
                _ => {
                    self.overlays.project.path_textarea.input(key_event);
                    self.overlays.project.error = None;
                }
            }
            return;
        }

        match key_event.code {
            KeyCode::Char('a') if key_event.modifiers == KeyModifiers::NONE => {
                self.start_project_path_input(DawProjectFileAction::SaveAs)
            }
            KeyCode::Char('o') if key_event.modifiers == KeyModifiers::NONE => {
                self.overlays.project.start_open_selector();
                if self.overlays.project.auto_preview {
                    self.preview_selected_project_file(false);
                }
            }
            KeyCode::Char('d') if key_event.modifiers == KeyModifiers::NONE => {
                let Some(config_app_dir) = self.config_app_dir.as_deref() else {
                    self.overlays.project.error =
                        Some("config app directory を取得できません".to_string());
                    return;
                };
                let archive_root = daily_archive_root(config_app_dir);
                if let Err(error) = std::fs::create_dir_all(&archive_root) {
                    self.overlays.project.error = Some(format!(
                        "Daily Archive directory を作成できません: {}; {error}",
                        archive_root.display()
                    ));
                    return;
                }
                self.overlays
                    .project
                    .start_daily_archive_selector(archive_root);
                if self.overlays.project.auto_preview {
                    self.preview_selected_project_file(false);
                }
            }
            KeyCode::Esc => {
                self.overlays.project.backup_notice_path = None;
                self.mode = DawMode::Normal;
            }
            _ => {}
        }
    }
}
