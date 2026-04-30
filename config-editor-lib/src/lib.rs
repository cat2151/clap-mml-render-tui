use serde::Deserialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ConfigEditorError>;

#[derive(Debug, Error)]
pub enum ConfigEditorError {
    #[error("config TOML を読み込めませんでした ({path}): {source}")]
    ReadConfig { path: PathBuf, source: io::Error },
    #[error("config TOML を解釈できませんでした ({path}): {source}")]
    ParseConfig {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },
    #[error("config の editors に有効な editor がありません")]
    NoValidEditors,
    #[error("利用できる editor が見つかりませんでした: {}", .attempted.join(", "))]
    NoEditorFound { attempted: Vec<String> },
    #[error("editor `{editor}` が終了コード {} で終了しました", format_exit_status(.status))]
    EditorFailed { editor: String, status: ExitStatus },
    #[error("editor `{editor}` を起動できませんでした: {source}")]
    SpawnEditor { editor: String, source: io::Error },
}

#[derive(Debug, Default, Deserialize)]
struct EditorConfig {
    editors: Option<Vec<String>>,
}

pub fn load_editors_or_default(
    config_path: impl AsRef<Path>,
    default_editors: &[&str],
) -> Result<Vec<String>> {
    let config_path = config_path.as_ref();
    let raw = fs::read_to_string(config_path).map_err(|source| ConfigEditorError::ReadConfig {
        path: config_path.to_path_buf(),
        source,
    })?;
    let config: EditorConfig =
        toml::from_str(&raw).map_err(|source| ConfigEditorError::ParseConfig {
            path: config_path.to_path_buf(),
            source: Box::new(source),
        })?;

    Ok(match config.editors {
        Some(editors) => normalize_editors(editors),
        None => default_editors
            .iter()
            .map(|editor| (*editor).to_string())
            .collect(),
    })
}

pub fn open_config_toml(config_path: impl AsRef<Path>, editors: &[String]) -> Result<()> {
    let config_path = config_path.as_ref();
    let mut attempted = Vec::new();

    for editor in editors.iter().map(|editor| editor.trim()) {
        if editor.is_empty() {
            continue;
        }
        attempted.push(editor.to_string());

        let mut command = Command::new(editor);
        command.args(editor_extra_args(editor));
        command.arg(config_path);

        match command.status() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => {
                return Err(ConfigEditorError::EditorFailed {
                    editor: editor.to_string(),
                    status,
                });
            }
            Err(source) if source.kind() == io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(ConfigEditorError::SpawnEditor {
                    editor: editor.to_string(),
                    source,
                });
            }
        }
    }

    if attempted.is_empty() {
        return Err(ConfigEditorError::NoValidEditors);
    }

    Err(ConfigEditorError::NoEditorFound { attempted })
}

fn normalize_editors(editors: Vec<String>) -> Vec<String> {
    editors
        .into_iter()
        .map(|editor| editor.trim().to_string())
        .filter(|editor| !editor.is_empty())
        .collect()
}

fn editor_extra_args(editor: &str) -> &'static [&'static str] {
    if editor.eq_ignore_ascii_case("code") {
        &["--wait"]
    } else {
        &[]
    }
}

fn format_exit_status(status: &ExitStatus) -> String {
    status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "不明".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cmrt-config-editor-{name}-{}-{unique}.toml",
            std::process::id()
        ))
    }

    #[test]
    fn load_editors_uses_app_default_when_key_is_missing() {
        let path = temp_config_path("missing");
        fs::write(&path, "sample_rate = 48000\n").unwrap();

        let editors = load_editors_or_default(&path, &["fresh", "code"]).unwrap();

        assert_eq!(editors, vec!["fresh".to_string(), "code".to_string()]);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn load_editors_trims_configured_values() {
        let path = temp_config_path("configured");
        fs::write(&path, r#"editors = [" fresh ", "", "code"]"#).unwrap();

        let editors = load_editors_or_default(&path, &["vim"]).unwrap();

        assert_eq!(editors, vec!["fresh".to_string(), "code".to_string()]);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn editor_extra_args_waits_for_code() {
        assert_eq!(editor_extra_args("code"), ["--wait"]);
        assert_eq!(editor_extra_args("CODE"), ["--wait"]);
        assert!(editor_extra_args("vim").is_empty());
    }

    #[test]
    fn open_config_toml_reports_empty_editor_list() {
        let path = temp_config_path("empty");
        fs::write(&path, "").unwrap();

        let error = open_config_toml(&path, &[" ".to_string()]).unwrap_err();

        assert!(matches!(error, ConfigEditorError::NoValidEditors));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn open_config_toml_reports_all_missing_editors() {
        let path = temp_config_path("missing-editor");
        fs::write(&path, "").unwrap();
        let editors = vec![
            "cmrt-editor-test-not-found-a".to_string(),
            "cmrt-editor-test-not-found-b".to_string(),
        ];

        let error = open_config_toml(&path, &editors).unwrap_err();

        assert!(matches!(
            error,
            ConfigEditorError::NoEditorFound { attempted }
                if attempted == vec![
                    "cmrt-editor-test-not-found-a".to_string(),
                    "cmrt-editor-test-not-found-b".to_string()
                ]
        ));
        fs::remove_file(path).unwrap();
    }
}
