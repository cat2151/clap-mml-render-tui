//! `cat-self-update-lib` を利用した self update 関連機能。

use anyhow::Result;
use cat_self_update_lib::check_remote_commit;

const REPO_OWNER: &str = "cat2151";
const REPO_NAME: &str = "clap-mml-render-tui";
const MAIN_BRANCH: &str = "main";
const APP_BIN_NAMES: &[&str] = &["cmrt"];

/// ビルド時に埋め込まれたgit commit hash
const LOCAL_HASH: &str = env!("GIT_COMMIT_HASH");

fn is_valid_sha1(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn validate_check_hash(local_hash: &'static str) -> Result<&'static str> {
    if local_hash == "unknown" || !is_valid_sha1(local_hash) {
        anyhow::bail!(
            "このビルドでは commit hash を取得できないため `cmrt check` を実行できません。git clone した作業ツリーからビルドし直してください。"
        );
    }

    Ok(local_hash)
}

/// `cmrt update` などの CLI サブコマンドから self update を開始する。
/// 必要に応じて、事前にサーバーや関連プロセスを停止した状態で呼び出すこと。
pub fn run_foreground_update() -> Result<()> {
    let (owner, repo, bins) = update_target();

    println!("アップデートを開始します...");
    cat_self_update_lib::self_update(owner, repo, bins)
        .map_err(|e| anyhow::anyhow!("アップデート開始に失敗しました: {}", e))?;
    println!("アップデートをバックグラウンドで開始しました。完了後に cmrt を再起動します。");

    Ok(())
}

/// ビルド時のコミットハッシュと remote main の先頭コミットを比較して表示する。
pub fn run_check() -> Result<()> {
    let (owner, repo, branch, local_hash) = check_target();
    let local_hash = validate_check_hash(local_hash)?;
    let result = check_remote_commit(owner, repo, branch, local_hash)
        .map_err(|e| anyhow::anyhow!("アップデート確認に失敗しました: {}", e))?;
    println!("{result}");
    Ok(())
}

fn update_target() -> (&'static str, &'static str, &'static [&'static str]) {
    (REPO_OWNER, REPO_NAME, APP_BIN_NAMES)
}

fn check_target() -> (&'static str, &'static str, &'static str, &'static str) {
    (REPO_OWNER, REPO_NAME, MAIN_BRANCH, LOCAL_HASH.trim())
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

    #[test]
    fn test_is_valid_sha1_accepts_40_hex_chars() {
        assert!(is_valid_sha1("0123456789abcdef0123456789abcdef01234567"));
    }

    #[test]
    fn test_is_valid_sha1_rejects_non_hex_or_wrong_length() {
        assert!(!is_valid_sha1("unknown"));
        assert!(!is_valid_sha1("0123456789abcdef0123456789abcdef0123456z"));
        assert!(!is_valid_sha1("0123456789abcdef"));
    }

    #[test]
    fn test_validate_check_hash_rejects_unknown() {
        let err = validate_check_hash("unknown").unwrap_err();
        assert!(err.to_string().contains(
            "このビルドでは commit hash を取得できないため `cmrt check` を実行できません。"
        ));
    }

    #[test]
    fn test_validate_check_hash_rejects_invalid_hash() {
        let err = validate_check_hash("not-a-sha1").unwrap_err();
        assert!(err.to_string().contains(
            "このビルドでは commit hash を取得できないため `cmrt check` を実行できません。"
        ));
    }

    #[test]
    fn test_validate_check_hash_accepts_valid_hash() {
        let hash = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(validate_check_hash(hash).unwrap(), hash);
    }

    #[test]
    fn test_check_target_returns_expected_values() {
        let (owner, repo, branch, local_hash) = check_target();

        assert_eq!(owner, "cat2151");
        assert_eq!(repo, "clap-mml-render-tui");
        assert_eq!(branch, "main");
        assert!(!local_hash.is_empty());
    }
}
