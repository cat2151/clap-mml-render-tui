//! 自動アップデート機能。
//! 起動時にGitHubのmainブランチのhashをチェックし、
//! ローカルのhashと異なる場合はユーザーの確認後にアップデートを実行する。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Result;

const REPO_OWNER: &str = "cat2151";
const REPO_NAME: &str = "clap-mml-render-tui";

/// ビルド時に埋め込まれたgit commit hash
const LOCAL_HASH: &str = env!("GIT_COMMIT_HASH");

/// アップデートチェックの最小間隔（1時間）
const CHECK_INTERVAL_SECS: u64 = 3600;

/// ローカルhashが有効なSHA-1の40文字16進数文字列かを確認する
fn is_valid_sha1(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// 最後にアップデートチェックを実行した時刻のファイルパスを返す
fn last_check_file() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|d| d.join("cmrt").join("last_update_check"))
}

/// 前回チェックから十分時間が経過していなければ `false` を返す（レート制限）
fn should_check_now() -> bool {
    let Some(path) = last_check_file() else {
        return true;
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return true;
    };
    let Ok(ts) = content.trim().parse::<u64>() else {
        return true;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(ts) >= CHECK_INTERVAL_SECS
}

/// 最後のチェック時刻を現在時刻で更新する
fn update_last_check_time() {
    let Some(path) = last_check_file() else {
        return;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, now.to_string());
}

/// リモートのmainブランチの最新commit hashをGitHub APIで取得する
fn fetch_remote_hash() -> Result<String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/commits/main",
        REPO_OWNER, REPO_NAME
    );

    let resp: serde_json::Value = ureq::AgentBuilder::new()
        .timeout_read(std::time::Duration::from_secs(10))
        .timeout_write(std::time::Duration::from_secs(10))
        .build()
        .get(&url)
        .set("User-Agent", "clap-mml-render-tui-updater")
        .set("Accept", "application/vnd.github.v3+json")
        .call()?
        .into_json()?;

    resp["sha"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("SHA field not found in GitHub API response"))
}

/// バックグラウンドでアップデートチェックを実行する。
/// 更新が必要な場合は `update_available` を true にセットする。
pub fn spawn_update_check(update_available: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        if let Err(_e) = check_for_update(update_available) {
            // TUI動作中のためeprintlnは使わない（表示崩れ防止）
        }
    });
}

fn check_for_update(update_available: Arc<AtomicBool>) -> Result<()> {
    // デバッグビルド時は自動アップデートをスキップ（開発中の誤更新を防止）
    if cfg!(debug_assertions) {
        return Ok(());
    }

    // ローカルhashが有効なSHA-1でなければスキップ（不明なビルド環境）
    let local = LOCAL_HASH.trim();
    if !is_valid_sha1(local) {
        return Ok(());
    }

    // レート制限：前回チェックから1時間以内はスキップする
    if !should_check_now() {
        return Ok(());
    }

    // チェック時刻を記録してから取得する（取得失敗でも次回まで待つ）
    update_last_check_time();

    // リモートhashを取得
    let remote_hash = match fetch_remote_hash() {
        Ok(h) => h,
        Err(_) => return Ok(()), // ネットワークエラーはサイレントに無視
    };

    // リモートhashがlocal hashと一致していれば何もしない
    if remote_hash == local {
        return Ok(());
    }

    // アップデートが利用可能: フラグをセット
    update_available.store(true, Ordering::Relaxed);

    Ok(())
}

/// フォアグラウンドでアップデートを実行する。
/// TUIを終了してから呼び出すこと。
pub fn run_foreground_update() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        println!("アップデートをバッチファイルで開始します...");
        spawn_updater_process()
            .map_err(|e| anyhow::anyhow!("バッチファイルアップデーターの起動に失敗しました: {}", e))?;
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("アップデートを開始します...");
        println!(
            "cargo install --force --git https://github.com/{}/{}",
            REPO_OWNER, REPO_NAME
        );

        let status = std::process::Command::new("cargo")
            .args([
                "install",
                "--force",
                "--git",
                &format!("https://github.com/{}/{}", REPO_OWNER, REPO_NAME),
            ])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()?;

        if status.success() {
            println!("アップデート成功！再起動します...");
            match std::process::Command::new("cmrt").spawn() {
                Ok(_) => {
                    // 新しいプロセスを起動したら現在のプロセスを終了する（二重起動を防ぐ）
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("cmrtの再起動に失敗しました: {}。手動で再起動してください。", e);
                }
            }
        } else {
            eprintln!("アップデートに失敗しました。");
        }

        Ok(())
    }
}

/// Windowsでのアップデートを行うバッチファイルをspawnする。
#[cfg(target_os = "windows")]
fn spawn_updater_process() -> Result<()> {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let script_path = std::env::temp_dir().join(format!("cmrt_updater_{}.bat", suffix));
    let script = format!(
        "@echo off\r\ntimeout /t 3 /nobreak >nul\r\ncargo install --force --git https://github.com/{}/{}\r\ncmrt\r\n(goto) 2>nul & del \"%~f0\"\r\n",
        REPO_OWNER, REPO_NAME
    );
    std::fs::write(&script_path, &script)?;
    let script_str = script_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Updater script path contains invalid UTF-8"))?;
    // スペースを含むパスに対応するため、スクリプトパスをダブルクォートで囲む
    std::process::Command::new("cmd")
        .args(["/C", "start", "cmrt updater", &format!("\"{}\"", script_str)])
        .spawn()?;
    Ok(())
}
