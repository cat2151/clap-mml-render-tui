use anyhow::{Context, Result};

pub(crate) fn restart_current_process() -> Result<()> {
    let exe = std::env::current_exe().context("現在の実行ファイルパスを取得できませんでした")?;
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let status = std::process::Command::new(&exe)
        .args(args)
        .status()
        .with_context(|| format!("アプリの再起動に失敗しました: {}", exe.display()))?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "再起動したアプリが終了コード {} で終了しました",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "不明".to_string())
        );
    }
}
