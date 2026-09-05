//! Daily DAW の復帰ファイル、日付判定、managed Archive。

use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use cmrt_history::DawCachedMeasure;
use serde::{Deserialize, Serialize};

use super::{
    project::{
        project_file_from_app, project_snapshot_for_recovery, validate_project_file_for_recovery,
        DawProjectFile,
    },
    DawApp,
};

const DAILY_FEATURE_DIRECTORY: &str = "daily_daw";
const DAILY_CURRENT_FILE_NAME: &str = "current.json";
const DAILY_ARCHIVE_DIRECTORY: &str = "archive";
const PROJECT_FILE_SUFFIX: &str = ".cmrt-daw.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DailyRecoveryFile {
    pub(crate) page_date: String,
    pub(crate) project_file: DawProjectFile,
    pub(crate) cursor_track: usize,
    pub(crate) cursor_measure: usize,
    pub(crate) cached_measures: Vec<DawCachedMeasure>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DailyDateClassification {
    FirstUse,
    Resume,
    Rollover,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DailyArchiveOutcome {
    Created,
    AlreadyExists,
}

pub(crate) fn daily_feature_root(config_app_dir: &Path) -> PathBuf {
    config_app_dir.join(DAILY_FEATURE_DIRECTORY)
}

pub(crate) fn daily_current_path(config_app_dir: &Path) -> PathBuf {
    daily_feature_root(config_app_dir).join(DAILY_CURRENT_FILE_NAME)
}

pub(crate) fn daily_archive_root(config_app_dir: &Path) -> PathBuf {
    daily_feature_root(config_app_dir).join(DAILY_ARCHIVE_DIRECTORY)
}

pub(crate) fn daily_archive_path(config_app_dir: &Path, page_date: &str) -> Result<PathBuf> {
    validate_local_date(page_date)?;
    Ok(daily_archive_root(config_app_dir).join(format!("{page_date}{PROJECT_FILE_SUFFIX}")))
}

pub(crate) fn load_daily_recovery(path: &Path) -> Result<Option<DailyRecoveryFile>> {
    let json = match std::fs::read_to_string(path) {
        Ok(json) => json,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Daily recovery を読めません: {}", path.display()));
        }
    };
    let recovery = decode_daily_recovery(&json)
        .with_context(|| format!("Daily recovery が不正です: {}", path.display()))?;
    Ok(Some(recovery))
}

pub(crate) fn write_daily_recovery(path: &Path, recovery: &DailyRecoveryFile) -> Result<()> {
    validate_local_date(&recovery.page_date)?;
    let mut bytes =
        serde_json::to_vec_pretty(recovery).context("Daily recovery JSON を生成できません")?;
    bytes.push(b'\n');
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Daily recovery directory を作成できません: {}",
                parent.display()
            )
        })?;
    }
    std::fs::write(path, bytes)
        .with_context(|| format!("Daily recovery を保存できません: {}", path.display()))
}

fn decode_daily_recovery(json: &str) -> Result<DailyRecoveryFile> {
    let recovery = serde_json::from_str::<DailyRecoveryFile>(json)
        .context("Daily recovery JSON を解釈できません")?;
    validate_local_date(&recovery.page_date)?;
    validate_project_file_for_recovery(&recovery.project_file)
        .context("Daily recovery 内の project が不正です")?;
    Ok(recovery)
}

pub(crate) fn classify_daily_date(
    page_date: Option<&str>,
    current_date: &str,
) -> Result<DailyDateClassification> {
    validate_local_date(current_date).context("current date が不正です")?;
    let Some(page_date) = page_date else {
        return Ok(DailyDateClassification::FirstUse);
    };
    validate_local_date(page_date).context("page_date が不正です")?;

    if current_date > page_date {
        Ok(DailyDateClassification::Rollover)
    } else {
        Ok(DailyDateClassification::Resume)
    }
}

pub(crate) fn write_daily_archive(
    path: &Path,
    project_file: &DawProjectFile,
) -> Result<DailyArchiveOutcome> {
    let mut bytes = serde_json::to_vec_pretty(project_file)
        .context("Daily Archive の project JSON を生成できません")?;
    bytes.push(b'\n');

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Daily Archive directory を作成できません: {}",
                parent.display()
            )
        })?;
    }

    create_new_archive_with(path, |file| {
        file.write_all(&bytes)?;
        file.flush()
    })
}

fn create_new_archive_with(
    path: &Path,
    write_contents: impl FnOnce(&mut File) -> std::io::Result<()>,
) -> Result<DailyArchiveOutcome> {
    let mut file = match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            return Ok(DailyArchiveOutcome::AlreadyExists);
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Daily Archive を作成できません: {}", path.display()));
        }
    };

    if let Err(write_error) = write_contents(&mut file) {
        drop(file);
        if let Err(cleanup_error) = std::fs::remove_file(path) {
            if cleanup_error.kind() != ErrorKind::NotFound {
                bail!(
                    "Daily Archive の書込に失敗し、不完全な file も削除できませんでした: {write_error}; path={}; cleanup error={cleanup_error}",
                    path.display()
                );
            }
        }
        bail!(
            "Daily Archive を書き込めません: {}; {write_error}",
            path.display()
        );
    }

    Ok(DailyArchiveOutcome::Created)
}

fn validate_local_date(date: &str) -> Result<()> {
    let bytes = date.as_bytes();
    let has_wire_shape = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit());
    if !has_wire_shape {
        bail!("local date は有効な YYYY-MM-DD である必要があります: {date}");
    }

    let year: u32 = date[0..4].parse().expect("validated ASCII digits");
    let month: u32 = date[5..7].parse().expect("validated ASCII digits");
    let day: u32 = date[8..10].parse().expect("validated ASCII digits");
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => 0,
    };
    if day == 0 || day > days_in_month {
        bail!("local date は有効な YYYY-MM-DD である必要があります: {date}");
    }
    Ok(())
}

impl DawApp {
    pub(crate) fn initialize_daily_workspace(&mut self, current_date: &str) {
        if let Err(error) = validate_local_date(current_date) {
            self.daily_page_date = Some(current_date.to_owned());
            self.append_log_line(format!("daily recovery failed: starting fresh: {error}"));
            self.sync_cache_states();
            return;
        }

        self.daily_page_date = Some(current_date.to_owned());
        let Some(config_app_dir) = self.config_app_dir.clone() else {
            self.sync_cache_states();
            return;
        };
        let current_path = daily_current_path(&config_app_dir);
        let recovery = match load_daily_recovery(&current_path) {
            Ok(Some(recovery)) => recovery,
            Ok(None) => {
                self.sync_cache_states();
                let _ = self.save_daily_recovery();
                return;
            }
            Err(error) => {
                self.append_log_line(format!(
                    "daily recovery failed: path={}; starting fresh: {error}",
                    current_path.display()
                ));
                self.sync_cache_states();
                let _ = self.save_daily_recovery();
                return;
            }
        };

        match classify_daily_date(Some(&recovery.page_date), current_date) {
            Ok(DailyDateClassification::Resume) => {
                if let Err(error) = self.apply_daily_recovery(recovery) {
                    self.append_log_line(format!(
                        "daily recovery failed: path={}; starting fresh: {error}",
                        current_path.display()
                    ));
                    self.daily_page_date = Some(current_date.to_owned());
                    let _ = self.save_daily_recovery();
                }
            }
            Ok(DailyDateClassification::Rollover) => {
                self.rollover_daily_recovery(recovery, current_date, &config_app_dir);
            }
            Ok(DailyDateClassification::FirstUse) => unreachable!("recovery has a page date"),
            Err(error) => {
                self.append_log_line(format!(
                    "daily recovery failed: path={}; starting fresh: {error}",
                    current_path.display()
                ));
                let _ = self.save_daily_recovery();
            }
        }
    }

    fn rollover_daily_recovery(
        &mut self,
        recovery: DailyRecoveryFile,
        current_date: &str,
        config_app_dir: &Path,
    ) {
        let previous_date = recovery.page_date.clone();
        let archive_path = match daily_archive_path(config_app_dir, &previous_date) {
            Ok(path) => path,
            Err(error) => {
                self.keep_daily_after_rollover_failure(
                    recovery,
                    current_date,
                    Path::new("<invalid>"),
                    error,
                );
                return;
            }
        };

        match write_daily_archive(&archive_path, &recovery.project_file) {
            Ok(DailyArchiveOutcome::Created | DailyArchiveOutcome::AlreadyExists) => {
                self.daily_page_date = Some(current_date.to_owned());
                self.sync_cache_states();
                let _ = self.save_daily_recovery();
                self.append_log_line(format!(
                    "daily rollover: {previous_date} -> {current_date}; archive={}",
                    archive_path.display()
                ));
                self.clear_daily_cache_after_rollover();
            }
            Err(error) => {
                self.keep_daily_after_rollover_failure(recovery, current_date, &archive_path, error)
            }
        }
    }

    /// rollover 成功後に、前日のキャッシュ WAV を捨てる。
    ///
    /// **呼んでよいのは archive の書き込みが `Ok` を返した腕だけ。**
    /// rollover 後のページは空（[`Self::apply_daily_recovery`] を呼ばない）なので、
    /// 掃除しないと前日の WAV が今日のセル名を占めたまま鳴り続ける。ファイル名に
    /// 日付も hash も入らないうえ、演奏ループはファイルの存在しか見ないため。
    ///
    /// 逆に [`Self::keep_daily_after_rollover_failure`] は前日のページを**復元する**
    /// 経路で、`restore_cache_from_metadata()` が前日の WAV を必要とする。
    /// あちらで消すと「archive も書けていないのにキャッシュも無い」状態を作る。
    fn clear_daily_cache_after_rollover(&mut self) {
        let line = match crate::cache::clear_workspace_cache_wavs(crate::WorkspaceKind::Daily) {
            Ok((dir, removed)) => {
                format!(
                    "daily cache cleared: dir={}; removed={removed} wav",
                    dir.display()
                )
            }
            Err(error) => format!("daily cache clear failed: {error}"),
        };
        self.append_log_line(line);
    }

    fn keep_daily_after_rollover_failure(
        &mut self,
        recovery: DailyRecoveryFile,
        current_date: &str,
        archive_path: &Path,
        error: anyhow::Error,
    ) {
        let previous_date = recovery.page_date.clone();
        if let Err(apply_error) = self.apply_daily_recovery(recovery) {
            self.daily_page_date = Some(current_date.to_owned());
            self.append_log_line(format!(
                "daily recovery failed: starting fresh after rollover error: {apply_error}"
            ));
            return;
        }
        self.append_log_line(format!(
            "daily rollover failed: archive={}; keeping {previous_date}: {error}",
            archive_path.display()
        ));
    }

    fn apply_daily_recovery(&mut self, recovery: DailyRecoveryFile) -> Result<()> {
        let snapshot = project_snapshot_for_recovery(&recovery.project_file)
            .context("Daily recovery project を適用できません")?;
        self.apply_project_snapshot_for_recovery(snapshot);
        self.daily_page_date = Some(recovery.page_date);
        self.editor.cursor_track = recovery.cursor_track.min(self.editor.tracks - 1);
        self.editor.cursor_measure = recovery.cursor_measure.min(self.editor.measures);
        self.restore_cache_from_metadata(&recovery.cached_measures);
        Ok(())
    }

    pub(crate) fn save_daily_recovery(&self) -> Result<()> {
        let config_app_dir = self
            .config_app_dir
            .as_deref()
            .context("config app directory を取得できません")?;
        let page_date = self
            .daily_page_date
            .clone()
            .context("Daily page date がありません")?;
        let recovery = DailyRecoveryFile {
            page_date,
            project_file: project_file_from_app(self),
            cursor_track: self.editor.cursor_track,
            cursor_measure: self.editor.cursor_measure,
            cached_measures: self.cached_measures_for_history(),
        };
        write_daily_recovery(&daily_current_path(config_app_dir), &recovery)
    }
}

#[cfg(test)]
mod tests;
