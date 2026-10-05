//! catalog 構築の処理内容を、実行前に表示して同期ログへ残す。

use std::io::Write as _;
use std::time::Instant;

use anyhow::Result;

pub fn report(message: impl AsRef<str>) {
    let message = message.as_ref();
    println!("{message}");
    let _ = std::io::stdout().flush();
    crate::logging::global_log_sink(&format!(
        "patch-catalog-build: pid={} {message}",
        std::process::id()
    ));
}

pub(super) fn run<T>(description: &str, work: impl FnOnce() -> Result<T>) -> Result<T> {
    run_with_report(description, work, report)
}

fn run_with_report<T>(
    description: &str,
    work: impl FnOnce() -> Result<T>,
    mut report: impl FnMut(String),
) -> Result<T> {
    report(format!("開始: {description}"));
    let started = Instant::now();
    let result = work();
    let elapsed = started.elapsed().as_secs_f64();
    match &result {
        Ok(_) => report(format!("完了: {description} ({elapsed:.2}秒)")),
        Err(error) => report(format!("失敗: {description} ({elapsed:.2}秒): {error:#}")),
    }
    result
}

#[cfg(test)]
mod tests;
