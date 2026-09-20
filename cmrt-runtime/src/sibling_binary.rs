//! ヘルパー実行ファイル（play server / render-server）の実体を
//! 「呼び出し元の実行ファイルと同じディレクトリ → 兄弟 repo の release」の順で探す、共通の探索本体。
//!
//! play server（`clap-mml-play-server`）は先に固定され、**PATH は見ない**。render-server も
//! 同じ順にする。呼び出し側ごとに違うのは「探す実行ファイル名」と「掴んだ後に何をするか」
//! （profile 判定・staleness 判定は play server の画面バッジだけが必要とする）だけなので、
//! ここでは持たない。

use std::path::{Path, PathBuf};

/// 兄弟 repo（play server 側）のディレクトリ名。render-server の実体もこの repo に居る。
///
/// 既存の python スクリプト（`scripts/capture_daw_live_mix.py` の `PLAY_SERVER_ROOT`）と
/// 同じ規則。綴りを変えるときは両方を揃えること。
pub const PLAY_SERVER_REPO_DIR_NAME: &str = "clap-mml-play-server";

/// 見つかった経路がどちらだったか。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SiblingBinarySource {
    /// 呼び出し元の実行ファイル（`cmrt.exe`）と同じディレクトリ。
    SiblingDirectory,
    /// 兄弟 repo の release ビルド。
    SiblingRepoRelease,
}

/// 見つかった実行ファイルの場所。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSiblingBinary {
    pub path: PathBuf,
    pub source: SiblingBinarySource,
}

/// `exe_name` を「`current_exe` と同じディレクトリ → 兄弟 repo `sibling_repo_dir_name` の
/// `target/release/`」の順で探す。**上から順に、最初に見つかったものを使う。PATH は見ない。**
///
/// 見つからなければ、探した場所の説明（日本語）を `Err` で返す。
pub fn resolve_sibling_binary(
    current_exe: Option<&Path>,
    exe_name: &str,
    sibling_repo_dir_name: &str,
) -> Result<ResolvedSiblingBinary, Vec<String>> {
    let mut searched = Vec::new();

    let sibling = current_exe.and_then(|exe| sibling_path(exe, exe_name));
    if let Some(path) = sibling {
        if path.is_file() {
            return Ok(ResolvedSiblingBinary {
                path,
                source: SiblingBinarySource::SiblingDirectory,
            });
        }
        searched.push(format!(
            "{} (実行ファイルと同じディレクトリ)",
            path.display()
        ));
    }

    let repo_release =
        current_exe.and_then(|exe| sibling_repo_release_path(exe, exe_name, sibling_repo_dir_name));
    if let Some(path) = repo_release {
        if path.is_file() {
            return Ok(ResolvedSiblingBinary {
                path,
                source: SiblingBinarySource::SiblingRepoRelease,
            });
        }
        searched.push(format!("{} (兄弟 repo の release)", path.display()));
    }

    if searched.is_empty() {
        searched.push("(自分自身の場所が取れませんでした)".to_owned());
    }
    Err(searched)
}

/// 見つからなかったことを説明する行。エラー文にも UI にもこれを流す。
///
/// `entity_label` は「play server」「render-server」など、探しているものの呼び名。
pub fn not_found_lines(entity_label: &str, exe_name: &str, searched: &[String]) -> Vec<String> {
    let mut lines = vec![format!(
        "{entity_label} の実体が見つかりません（{exe_name}）"
    )];
    lines.extend(searched.iter().map(|place| format!("探した場所: {place}")));
    lines
}

fn sibling_path(current_exe: &Path, exe_name: &str) -> Option<PathBuf> {
    Some(current_exe.parent()?.join(exe_name))
}

/// `<root>/target/<profile>/<exe>` の `<root>` から
/// `<root の親>/<sibling_repo_dir_name>/target/release/<exe_name>`。
/// **`target/debug` か `target/release` に置かれているときだけ**遡る。
/// つまりこの経路は開発ビルドのときにしか効かず、配布物では必ず同じディレクトリで決まる。
fn sibling_repo_release_path(
    current_exe: &Path,
    exe_name: &str,
    sibling_repo_dir_name: &str,
) -> Option<PathBuf> {
    let profile_dir = current_exe.parent()?;
    if !matches!(profile_dir.file_name()?.to_str()?, "debug" | "release") {
        return None;
    }
    let target_dir = profile_dir.parent()?;
    if target_dir.file_name()?.to_str()? != "target" {
        return None;
    }
    let repo_root = target_dir.parent()?;
    Some(
        repo_root
            .parent()?
            .join(sibling_repo_dir_name)
            .join("target")
            .join("release")
            .join(exe_name),
    )
}

#[cfg(test)]
mod tests;
