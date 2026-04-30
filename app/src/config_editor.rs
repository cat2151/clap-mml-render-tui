use anyhow::{anyhow, Result};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use serde::Serialize;
use std::{io::Stdout, path::Path};

pub(crate) const DEFAULT_CONFIG_EDITORS: &[&str] = &["fresh", "zed", "code", "edit", "nano", "vim"];

pub(crate) type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

#[derive(Serialize)]
struct EditorsToml<'a> {
    editors: &'a [&'a str],
}

pub(crate) fn default_config_editor_block() -> String {
    format!(
        "# 【省略可】config.toml を開く editor 候補（左から順に試す）\n{}\n",
        default_config_editors_line()
    )
}

pub(crate) fn default_config_editors_line() -> String {
    toml::to_string(&EditorsToml {
        editors: DEFAULT_CONFIG_EDITORS,
    })
    .unwrap_or_else(|_| r#"editors = ["fresh", "zed", "code", "edit", "nano", "vim"]"#.into())
    .trim()
    .to_string()
}

pub(crate) fn configured_editors(config_path: impl AsRef<Path>) -> Result<Vec<String>> {
    cmrt_config_editor::load_editors_or_default(config_path, DEFAULT_CONFIG_EDITORS)
        .map_err(Into::into)
}

pub(crate) fn edit_config_toml(terminal: &mut AppTerminal) -> Result<()> {
    let config_path = crate::config::config_file_path().ok_or_else(|| {
        anyhow!("システムの設定ディレクトリが取得できないため config.toml を開けません")
    })?;
    let editors = configured_editors(&config_path)?;

    suspend_terminal(terminal)?;
    let edit_result = cmrt_config_editor::open_config_toml(&config_path, &editors);
    let resume_result = resume_terminal(terminal);

    match (edit_result, resume_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(edit_error), Ok(())) => Err(edit_error.into()),
        (Ok(()), Err(resume_error)) => Err(resume_error),
        (Err(edit_error), Err(resume_error)) => Err(anyhow!(
            "{edit_error}; TUI への復帰にも失敗しました: {resume_error}"
        )),
    }
}

fn suspend_terminal(terminal: &mut AppTerminal) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn resume_terminal(terminal: &mut AppTerminal) -> Result<()> {
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;
    Ok(())
}
