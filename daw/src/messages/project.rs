//! Project File overlay でユーザーへ直接表示する文言。

pub(crate) const OVERLAY_TITLE: &str = " Project File ";

pub(crate) const BACKUP_RENAMED: &str = "既存のプロジェクトファイルを次の名前に変更しました:";
pub(crate) const BACKUP_CREATED_TITLE: &str = " Backup created ";

pub(crate) const CURRENT_PATH_UNSET: &str = "(not opened or saved yet)";
pub(crate) const PROJECT_FILE_DESCRIPTION: &str =
    "DAW grid and mixer volumes are stored in one JSON file.";
pub(crate) const SAVE_AS_ACTION: &str = "  Save As";
pub(crate) const OPEN_ACTION: &str = "  Open";
pub(crate) const OPEN_DAILY_ARCHIVE_ACTION: &str = "  Open Daily Archive";
pub(crate) const CLOSE_ACTION: &str = "  Close";
pub(crate) const CURRENT_PATH_LABEL: &str = "Current: ";

pub(crate) const SAVE_AS_PATH_TITLE: &str = "Save As path";
pub(crate) const SAVE_AS_PATH_DESCRIPTION: &str =
    "Save As: absolute or current-directory-relative path";
pub(crate) const SAVE_AS_PATH_PLACEHOLDER: &str = "project.cmrt-daw.json";
pub(crate) const SAVE_AS_FOOTER: &str = "Enter: execute  ESC: back";

pub(crate) const OPEN_DESCRIPTION: &str = "Open: *.cmrt-daw.json";
pub(crate) const FILTER_ACTIVE_TITLE: &str = " Project filter (Enter=確定 / ESC=中断) ";
pub(crate) const FILTER_TITLE: &str = " Project filter ";
pub(crate) const FILTER_PLACEHOLDER: &str = "/ で filename 絞り込み";
pub(crate) const FILE_SELECTOR_UNAVAILABLE: &str = "file selector is unavailable";
pub(crate) const FILTER_ACTIVE_FOOTER: &str = "Enter:filter確定  ESC:filter中断  文字:filter入力";
pub(crate) const OPEN_FOOTER: &str =
    "/:filter  j/k:select  h/l:dir  Enter:open  Space:preview  a:auto  ESC:back";

pub(crate) const PREVIEW_MODE_AUTO: &str = "Auto";
pub(crate) const PREVIEW_MODE_MANUAL: &str = "Manual";
pub(crate) const PREVIEW_MODE_LABEL: &str = "Mode: ";
pub(crate) const NO_SELECTION: &str = "(none)";
pub(crate) const AUTO_PREVIEW_GUIDE: &str = "Select a project to preview.";
pub(crate) const MANUAL_PREVIEW_GUIDE: &str = "Space starts the selected preview.";
pub(crate) const DIRECTORY_PREVIEW: &str = "directory";
pub(crate) const NO_PLAYABLE_MEASURE: &str = "no playable measure";
pub(crate) const PLAYBACK_ACTIVE_PREVIEW_SKIPPED: &str = "full playback is active; preview skipped";

pub(crate) fn preview_title(mode: &str) -> String {
    format!(" Preview: {mode} ")
}

pub(crate) fn preview_measure(measure_index: usize) -> String {
    format!("meas{}", measure_index + 1)
}

pub(crate) fn preview_summary(tracks: usize, measures: usize, measure_label: &str) -> String {
    format!("tracks: {tracks}  measures: {measures}\npreview: {measure_label}")
}

pub(crate) fn project_directory_unreadable(error: &dyn std::fmt::Display) -> String {
    format!("project directory を読めません: {error}")
}

pub(crate) const STATUS_FILTER: &str = "PROJECT OPEN FILTER  Enter:確定  ESC:中断  文字:入力";
pub(crate) const STATUS_OPEN: &str =
    "PROJECT OPEN  j/k:選択  h/l:directory  /:filter  Enter:open  Space:preview  a:auto/manual  ESC:戻る";
pub(crate) const STATUS_SAVE_AS: &str = "PROJECT SAVE AS  Enter:保存  ESC:戻る";
pub(crate) const STATUS_MENU: &str = "PROJECT  a:Save As  o:Open  d:Open Daily Archive  ESC:閉じる";

pub(crate) const HELP_MENU: &str =
    "  f      : project file (a: Save As / o: Open / d: Open Daily Archive)";
pub(crate) const HELP_OPEN: &str =
    "           Open: j/k 選択, h/l directory, / filter, Space preview";
