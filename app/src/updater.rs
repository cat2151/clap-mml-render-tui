//! 自動アップデート機能。
//! `cat-self-update-lib` を利用して更新確認と self update を行う。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Result;
use cat_self_update_lib::check_remote_commit;

const REPO_OWNER: &str = "cat2151";
const REPO_NAME: &str = "clap-mml-render-tui";
const MAIN_BRANCH: &str = "main";
const APP_BIN_NAMES: &[&str] = &["cmrt"];

/// ビルド時に埋め込まれたgit commit hash
const LOCAL_HASH: &str = env!("GIT_COMMIT_HASH");

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

    if check_remote_commit(REPO_OWNER, REPO_NAME, MAIN_BRANCH, LOCAL_HASH.trim())
        .map_err(|e| anyhow::anyhow!("アップデート確認に失敗しました: {}", e))?
        .is_up_to_date()
    {
        return Ok(());
    }

    // アップデートが利用可能: フラグをセット
    update_available.store(true, Ordering::Relaxed);

    Ok(())
}

/// フォアグラウンドでアップデートを実行する。
/// TUIを終了してから呼び出すこと。
pub fn run_foreground_update() -> Result<()> {
    let (owner, repo, bins) = update_target();

    println!("アップデートを開始します...");
    cat_self_update_lib::self_update(owner, repo, bins)
        .map_err(|e| anyhow::anyhow!("アップデート開始に失敗しました: {}", e))?;
    println!("アップデートをバックグラウンドで開始しました。完了後に cmrt を再起動します。");

    Ok(())
}

fn update_target() -> (&'static str, &'static str, &'static [&'static str]) {
    (REPO_OWNER, REPO_NAME, APP_BIN_NAMES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_target_returns_correct_values() {
        let (owner, repo, bins) = update_target();

        assert!(!owner.is_empty());
        assert!(!repo.is_empty());
        assert!(!bins.is_empty());
        assert!(bins.iter().all(|bin| !bin.is_empty()));
        assert_eq!(
            (owner, repo, bins),
            ("cat2151", "clap-mml-render-tui", &["cmrt"] as &[&str])
        );
    }

    #[test]
    fn test_update_check_target_branch_is_main() {
        assert_eq!(MAIN_BRANCH, "main");
    }
}
