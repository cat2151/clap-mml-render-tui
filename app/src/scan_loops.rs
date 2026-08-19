//! `cmrt scan-loops`: loop_dirs を走査して WAV ループキャッシュを再構築する。
//!
//! 走査そのものは `cmrt-loop-browser-domain` が持つ。ここは進捗の画面出力と、
//! 途中経過を残す永続ログ（[`crate::scan_progress_log`]）への橋渡しだけ。

use anyhow::{Context, Result};

use clap_mml_render_tui::loop_browser::library as loop_library;

use crate::scan_progress_log;

pub(crate) fn write_scan_progress(
    event: &loop_library::LoopScanProgress,
    stdout: &mut dyn std::io::Write,
    stderr: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    match event {
        loop_library::LoopScanProgress::Started { roots } => {
            writeln!(stdout, "WAVループ走査を開始します: {roots} roots")?;
            stdout.flush()
        }
        loop_library::LoopScanProgress::Analyzing {
            current,
            total,
            path,
        } => {
            writeln!(stdout, "[{current}/{total}] WAVを解析: {}", path.display())?;
            stdout.flush()
        }
        loop_library::LoopScanProgress::Visualizing { .. } => Ok(()),
        loop_library::LoopScanProgress::Skipped { path, error } => {
            writeln!(
                stderr,
                "警告: WAVをスキップしました: {}\n  {error}",
                path.display()
            )?;
            stderr.flush()
        }
    }
}

pub(crate) fn write_scan_summary(
    summary: loop_library::LoopScanSummary,
    stdout: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    writeln!(
        stdout,
        "ループキャッシュを更新しました: {} roots / {} indexed WAV / {} skipped WAV",
        summary.roots, summary.wav_files, summary.skipped_wav_files
    )?;
    stdout.flush()
}

pub(crate) fn run_scan_loops(cfg: &clap_mml_render_tui::config::Config) -> Result<()> {
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();
    let mut output_error = None;
    let log_path = clap_mml_render_tui::config::scan_loops_log_file_path()
        .ok_or_else(|| anyhow::anyhow!("scan-loops logの保存先を取得できません"))?;
    let mut progress_log =
        scan_progress_log::ScanProgressLog::start(&log_path, std::time::Duration::from_secs(1))
            .with_context(|| format!("scan-loops logを開始できません: {}", log_path.display()))?;
    let scan_result = loop_library::scan_and_save_with_progress(&cfg.loop_dirs, |event| {
        progress_log.observe(&event);
        if output_error.is_none() {
            output_error = write_scan_progress(&event, &mut stdout, &mut stderr).err();
        }
    });
    let summary = match scan_result {
        Ok(summary) => summary,
        Err(error) => {
            let _ = progress_log.fail(&error);
            return Err(error);
        }
    };
    if let Some(error) = output_error {
        let _ = progress_log.fail(&error);
        return Err(error).context("scan-loopsの進捗を出力できません");
    }
    if let Err(error) = write_scan_summary(summary, &mut stdout) {
        let _ = progress_log.fail(&error);
        return Err(error).context("scan-loopsの完了結果を出力できません");
    }
    progress_log
        .finish(summary)
        .context("scan-loops logを完了できません")?;
    Ok(())
}

#[cfg(test)]
mod tests;
