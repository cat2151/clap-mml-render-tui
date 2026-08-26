use std::path::PathBuf;

use ratatui::{
    style::Style,
    widgets::{Block, Borders},
};
use ratatui_explorer::{FileExplorer, FileExplorerBuilder, Theme};
use ratatui_textarea::TextArea;

use super::super::DawProjectFileAction;
use crate::messages::project as message;
use cmrt_tui_core::theme::{MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_YELLOW};

const PROJECT_FILE_SUFFIX: &str = ".cmrt-daw.json";

pub(crate) struct DawProjectOverlayState {
    pub(crate) action: Option<DawProjectFileAction>,
    pub(crate) path_textarea: TextArea<'static>,
    pub(crate) file_explorer: Option<FileExplorer>,
    pub(crate) query: String,
    pub(crate) query_textarea: TextArea<'static>,
    pub(crate) query_before_input: String,
    pub(crate) filter_active: bool,
    pub(crate) auto_preview: bool,
    pub(crate) previewed_path: Option<PathBuf>,
    pub(crate) preview_info: Option<String>,
    pub(crate) preview_error: Option<String>,
    pub(crate) current_path: Option<PathBuf>,
    pub(crate) backup_notice_path: Option<PathBuf>,
    pub(crate) error: Option<String>,
}

impl DawProjectOverlayState {
    pub(crate) fn new() -> Self {
        Self {
            action: None,
            path_textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            file_explorer: None,
            query: String::new(),
            query_textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            query_before_input: String::new(),
            filter_active: false,
            auto_preview: true,
            previewed_path: None,
            preview_info: None,
            preview_error: None,
            current_path: None,
            backup_notice_path: None,
            error: None,
        }
    }

    pub(crate) fn start_open_selector(&mut self) {
        self.start_open_selector_from(DawProjectFileAction::Open, None);
    }

    pub(crate) fn start_daily_archive_selector(&mut self, archive_root: PathBuf) {
        self.start_open_selector_from(DawProjectFileAction::OpenDailyArchive, Some(archive_root));
    }

    fn start_open_selector_from(
        &mut self,
        action: DawProjectFileAction,
        working_dir: Option<PathBuf>,
    ) {
        self.query.clear();
        self.query_textarea = cmrt_tui_core::text_input::new_single_line_textarea("");
        self.query_before_input.clear();
        self.filter_active = false;
        self.previewed_path = None;
        self.preview_info = None;
        self.preview_error = None;
        let theme = Theme::new()
            .with_block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MONOKAI_CYAN)),
            )
            .add_default_title()
            .with_style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG))
            .with_item_style(Style::default().fg(MONOKAI_FG))
            .with_dir_style(Style::default().fg(MONOKAI_CYAN))
            .with_highlight_item_style(Style::default().fg(MONOKAI_BG).bg(MONOKAI_YELLOW))
            .with_highlight_dir_style(Style::default().fg(MONOKAI_BG).bg(MONOKAI_CYAN))
            .with_highlight_symbol("▶ ")
            .with_scroll_padding(1);
        let mut builder = FileExplorerBuilder::default()
            .theme(theme)
            .filter_map(project_file_filter(""));
        if let Some(working_dir) = working_dir {
            builder = builder.working_dir(working_dir);
        } else if let Some(current_path) = self.current_path.as_ref().filter(|path| path.exists()) {
            builder = builder.working_file(current_path.clone());
        }

        match builder.build() {
            Ok(file_explorer) => {
                self.file_explorer = Some(file_explorer);
                self.error = None;
            }
            Err(error) => {
                self.file_explorer = None;
                self.error = Some(message::project_directory_unreadable(&error));
            }
        }
        self.action = Some(action);
    }

    pub(crate) fn selected_path(&self) -> Option<PathBuf> {
        let explorer = self.file_explorer.as_ref()?;
        explorer
            .files()
            .get(explorer.selected_idx())
            .map(|file| file.path.clone())
    }

    pub(crate) fn apply_query(&mut self) -> std::io::Result<()> {
        if let Some(explorer) = self.file_explorer.as_mut() {
            explorer.set_filter_map(project_file_filter(&self.query))?;
            let first_file = explorer.files().iter().position(|file| !file.is_dir);
            if let Some(index) = first_file {
                explorer.set_selected_idx(index);
            }
        }
        Ok(())
    }
}

fn project_file_filter(
    query: &str,
) -> impl Fn(ratatui_explorer::File) -> Option<ratatui_explorer::File> {
    let terms = query
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    move |file| {
        if file.is_dir {
            return Some(file);
        }
        let name = file.name.to_lowercase();
        (name.ends_with(PROJECT_FILE_SUFFIX) && terms.iter().all(|term| name.contains(term)))
            .then_some(file)
    }
}
