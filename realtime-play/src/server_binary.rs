//! play server の実体をどれにするかの決定と、その素性（profile）の判定。
//!
//! 背景: 起動用 bat が PATH の先頭を切り替えるだけで debug / release が決まり、
//! アプリも画面もそれを知らなかった。debug サーバーは先読み（state load）が
//! 4〜5 倍遅く、小節の頭が無音になる「ぶつ切り」として現れたが、どちらの実体が
//! 動いているかはログの 1 行にしか出ていなかった。
//! 詳細は `docs/adr/0017-play-server-binary-resolution.md`。
//!
//! ここが持つのは 2 つ。
//! - **実体の決め方**（[`resolve_server_binary`]）: 上から順に、最初に見つかったものを使う。
//!   **PATH は見ない**（見ないことがこのモジュールの目的）
//! - **profile の判定**（[`ServerProfile`]）: 画面とログが同じ判定を使えるよう 1 か所に置く

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use cmrt_runtime::PlayServerLaunch;

/// 兄弟 repo（play server 側）のディレクトリ名。
///
/// 既存の python スクリプト（`scripts/capture_daw_live_mix.py` の `PLAY_SERVER_ROOT`）と
/// 同じ規則。綴りを変えるときは両方を揃えること。
const PLAY_SERVER_REPO_DIR_NAME: &str = "clap-mml-play-server";

/// 新しさの判定で見るファイル数の上限。
///
/// `target` を外してあるので実際は数百件で終わる。上限は「壊れた配置で固まらない」ための蓋。
const MAX_SCANNED_ENTRIES: usize = 20_000;

/// 掴んだ実体の素性。**画面もログもこの判定だけを使う。**
///
/// 判定材料はパスと「どの経路で決まったか」の 2 つしかない。cargo が作る
/// `target/debug` / `target/release` を含むかを先に見るのは、禁止されている
/// 「`./target/debug` へ手で cp する」をやってしまったときにも debug と言えるようにするため。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerProfile {
    /// `target/debug` 配下。先読みが 4〜5 倍遅く、演奏がぶつ切りになる。
    Debug,
    /// `target/release` 配下。開発機の通常運転。
    Release,
    /// `cmrt.exe` と同じディレクトリにある実体（= 配布物の通常形）。
    ///
    /// 配布物のパスは `target/` を含まないので profile を名乗れないが、
    /// 同梱された実体である以上、掴み間違いではない。
    Bundled,
    /// 上のどれでもない。何を掴んだのか分からないので、画面で目立たせる。
    Unknown,
}

impl ServerProfile {
    /// ログと画面に出す短い語。
    pub fn label(self) -> &'static str {
        match self {
            ServerProfile::Debug => "debug",
            ServerProfile::Release => "release",
            ServerProfile::Bundled => "同梱",
            ServerProfile::Unknown => "不明",
        }
    }

    /// 通常運転ではないので画面で目立たせるか。
    ///
    /// `release` と `同梱` は静かでよい。ここを広げると配布物でも警告が出っぱなしになり、
    /// 出っぱなしの警告は読まれなくなる。
    pub fn needs_attention(self) -> bool {
        matches!(self, ServerProfile::Debug | ServerProfile::Unknown)
    }
}

/// 実体が「どれで決まったか」。ログの `source=` がそのままこれ。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerSource {
    /// `cmrt --play-server <PATH>` で渡されたフルパス。
    Argument,
    /// `cmrt.exe` と同じディレクトリ。
    SiblingDirectory,
    /// 兄弟 repo の release ビルド。
    PlayServerRepoRelease,
    /// テストが立てる偽サーバー（shell 経由）。
    ShellCommand,
}

impl ServerSource {
    pub fn label(self) -> &'static str {
        match self {
            ServerSource::Argument => "argument",
            ServerSource::SiblingDirectory => "sibling",
            ServerSource::PlayServerRepoRelease => "play-server-repo-release",
            ServerSource::ShellCommand => "shell-command",
        }
    }
}

/// 実体より新しいソースが見つかったこと。
///
/// この ADR で PATH 解決を潰した代わりに生まれた穴がこれ。
/// 「兄弟 repo を直して debug だけ建て、TUI は古い release を掴み続ける」が
/// `release` = 通常運転なのでバッジも点かず、「直したのに変わらない」として現れていた。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaleSource {
    /// 実体より新しかったソースのうち、いちばん新しいもの。
    pub newest_source: String,
    /// 実体より何秒新しいか。
    pub newer_by_seconds: u64,
}

/// 決まった実体。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedServer {
    /// 掴んだ実体。フルパス、または（テストの偽サーバーなら）コマンド文字列。
    pub exe: String,
    pub source: ServerSource,
    pub profile: ServerProfile,
    /// 実体より新しいソースがあれば、そのいちばん新しいもの。
    ///
    /// 比べる相手は**この実体が置かれている `target/` を持つ cargo workspace**。
    /// `target/` の外に居る実体（配布物・テストの偽サーバー）では判定しない。
    pub stale: Option<StaleSource>,
}

impl ResolvedServer {
    /// ログ 1 行ぶんの key=value 列。
    pub fn log_fields(&self) -> String {
        let stale = match &self.stale {
            Some(stale) => format!(
                " stale_by_s={} newest_source={:?}",
                stale.newer_by_seconds, stale.newest_source
            ),
            None => String::new(),
        };
        format!(
            "source={} profile={} fullpath={:?}{stale}",
            self.source.label(),
            self.profile.label(),
            self.exe
        )
    }

    /// 画面で目立たせるべきか。**素性と新しさの両方**を見る。
    pub fn needs_attention(&self) -> bool {
        self.profile.needs_attention() || self.stale.is_some()
    }
}

/// 実体の決定結果。見つからなかったときも「どこを探したか」を返す。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerBinary {
    Resolved(ResolvedServer),
    /// どこにも無かった。素の実行ファイル名で spawn して OS のエラーに任せるのは、
    /// 「どこを探したのか」が誰にも分からなくなるのでやらない。
    NotFound {
        searched: Vec<String>,
    },
}

impl ServerBinary {
    pub fn resolved(&self) -> Option<&ResolvedServer> {
        match self {
            ServerBinary::Resolved(resolved) => Some(resolved),
            ServerBinary::NotFound { .. } => None,
        }
    }

    /// 画面で目立たせるべき実体か。見つからないときは起動時に別途エラーが出るので false。
    pub fn needs_attention(&self) -> bool {
        self.resolved().is_some_and(ResolvedServer::needs_attention)
    }

    /// 見つからなかったことを説明する行。エラー文にも UI にもこれを流す。
    pub fn not_found_lines(searched: &[String]) -> Vec<String> {
        let mut lines = vec![format!(
            "play server の実体が見つかりません（{}）",
            default_realtime_play_server_executable_name()
        )];
        lines.extend(searched.iter().map(|place| format!("探した場所: {place}")));
        lines
    }
}

pub fn default_realtime_play_server_executable_name() -> &'static str {
    if cfg!(windows) {
        "clap-mml-realtime-play-server.exe"
    } else {
        "clap-mml-realtime-play-server"
    }
}

/// 実体を決める。**上から順に、最初に見つかったものを使う。**
///
/// 1. 明示指定（`--play-server <PATH>` / テストの偽サーバー）
/// 2. `cmrt.exe` と同じディレクトリ
/// 3. 兄弟 repo の release
///
/// PATH は見ない。復活させないこと（ADR 0017）。
pub fn resolve_server_binary(launch_override: Option<&PlayServerLaunch>) -> ServerBinary {
    resolve_with(launch_override, std::env::current_exe().ok().as_deref())
}

/// [`resolve_server_binary`] の本体。`current_exe` を差し替えられる形にしてあるのは、
/// 探索のテストがユーザーの実環境に依存しないようにするため。
pub(crate) fn resolve_with(
    launch_override: Option<&PlayServerLaunch>,
    current_exe: Option<&Path>,
) -> ServerBinary {
    match launch_override {
        Some(PlayServerLaunch::ShellCommand(command)) => {
            return ServerBinary::Resolved(ResolvedServer {
                profile: classify(command, ServerSource::ShellCommand),
                exe: command.clone(),
                source: ServerSource::ShellCommand,
                stale: None,
            });
        }
        Some(PlayServerLaunch::Executable(path)) => {
            // 明示指定は探索へ落とさない。打った指定が黙って無視されるのは、
            // この ADR が潰した事故（環境が黙って実体を決める）と同じ手触りになる。
            return if path.is_file() {
                ServerBinary::Resolved(resolved_path(path, ServerSource::Argument))
            } else {
                ServerBinary::NotFound {
                    searched: vec![format!("{} (--play-server)", path.display())],
                }
            };
        }
        None => {}
    }

    let mut searched = Vec::new();

    let sibling = current_exe.and_then(sibling_server_path);
    if let Some(path) = sibling {
        if path.is_file() {
            return ServerBinary::Resolved(resolved_path(&path, ServerSource::SiblingDirectory));
        }
        searched.push(format!("{} (cmrt と同じディレクトリ)", path.display()));
    }

    let repo_release = current_exe.and_then(play_server_repo_release_path);
    if let Some(path) = repo_release {
        if path.is_file() {
            return ServerBinary::Resolved(resolved_path(
                &path,
                ServerSource::PlayServerRepoRelease,
            ));
        }
        searched.push(format!("{} (兄弟 repo の release)", path.display()));
    }

    if searched.is_empty() {
        searched.push("(cmrt 自身の場所が取れませんでした)".to_owned());
    }
    ServerBinary::NotFound { searched }
}

fn resolved_path(path: &Path, source: ServerSource) -> ResolvedServer {
    let exe = path.display().to_string();
    ResolvedServer {
        profile: classify(&exe, source),
        stale: stale_source(path),
        exe,
        source,
    }
}

/// 実体より新しいソースを 1 件だけ探す。無ければ `None`。
///
/// 比べる相手は**この実体が置かれている `target/` を持つ cargo workspace**。
/// その実体を建てたのがそのソースなので、他所の repo を持ち出さずに済む。
/// `target/` の外に居る実体（配布物・テストの偽サーバー）は判定しない
/// （比べるソースが無いし、配布物で毎回ファイル走査をしても得るものが無い）。
fn stale_source(exe: &Path) -> Option<StaleSource> {
    let workspace_root = cargo_workspace_root(exe)?;
    let built_at = exe.metadata().ok()?.modified().ok()?;
    let (newest_source, modified_at) = newest_source_after(workspace_root, built_at)?;
    let newer_by_seconds = modified_at
        .duration_since(built_at)
        .ok()
        .map_or(0, |elapsed| elapsed.as_secs());
    Some(StaleSource {
        newest_source: newest_source.display().to_string(),
        newer_by_seconds,
    })
}

/// `<root>/target/<profile>/<exe>` の `<root>`。その形でなければ `None`。
fn cargo_workspace_root(exe: &Path) -> Option<&Path> {
    let profile_dir = exe.parent()?;
    if !matches!(profile_dir.file_name()?.to_str()?, "debug" | "release") {
        return None;
    }
    let target_dir = profile_dir.parent()?;
    (target_dir.file_name()?.to_str()? == "target").then(|| target_dir.parent())?
}

/// `built_at` より新しいソースのうち、いちばん新しいもの。
///
/// 見るのは `*.rs` と `Cargo.toml` / `Cargo.lock` だけ。README を直しただけで
/// 「古い」と言われると、点きっぱなしの警告になって読まれなくなる。
fn newest_source_after(
    workspace_root: &Path,
    built_at: SystemTime,
) -> Option<(PathBuf, SystemTime)> {
    let mut newest: Option<(PathBuf, SystemTime)> = None;
    let mut directories = vec![workspace_root.to_path_buf()];
    let mut visited = 0usize;

    while let Some(directory) = directories.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            visited += 1;
            if visited > MAX_SCANNED_ENTRIES {
                // 走査は起動時の 1 回きりだが、上限は置く。ここで数秒使うくらいなら
                // 「古いかどうか分からない」と黙るほうがよい。
                return newest;
            }
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                if !is_skipped_directory(&path) {
                    directories.push(path);
                }
                continue;
            }
            if !is_source_file(&path) {
                continue;
            }
            let Some(modified_at) = entry.metadata().ok().and_then(|meta| meta.modified().ok())
            else {
                continue;
            };
            if modified_at <= built_at {
                continue;
            }
            if newest.as_ref().is_none_or(|(_, seen)| modified_at > *seen) {
                newest = Some((path, modified_at));
            }
        }
    }
    newest
}

/// 走査から外すディレクトリ。`target` を外さないと、成果物を数万件なめることになる。
fn is_skipped_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "target" | "node_modules") || name.starts_with('.'))
}

fn is_source_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name == "Cargo.toml" || name == "Cargo.lock" || name.ends_with(".rs")
}

fn sibling_server_path(current_exe: &Path) -> Option<PathBuf> {
    Some(
        current_exe
            .parent()?
            .join(default_realtime_play_server_executable_name()),
    )
}

/// 兄弟 repo の release ビルドのパス。
///
/// 「この repo の親 / clap-mml-play-server」。repo root は `cmrt.exe` の場所から求めるが、
/// **`target/debug` か `target/release` に置かれているときだけ**遡る。
/// つまりこの経路は開発ビルドのときにしか効かず、配布物では必ず 2 番（同じディレクトリ）で決まる。
fn play_server_repo_release_path(current_exe: &Path) -> Option<PathBuf> {
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
            .join(PLAY_SERVER_REPO_DIR_NAME)
            .join("target")
            .join("release")
            .join(default_realtime_play_server_executable_name()),
    )
}

/// パスと経路から profile を決める。**画面もログもここだけを通る。**
fn classify(path: &str, source: ServerSource) -> ServerProfile {
    if has_target_profile_dir(path, "debug") {
        ServerProfile::Debug
    } else if has_target_profile_dir(path, "release") {
        ServerProfile::Release
    } else if source == ServerSource::SiblingDirectory {
        ServerProfile::Bundled
    } else {
        ServerProfile::Unknown
    }
}

/// パスが `target/<profile>/` を含むか。`\` と `/` のどちらの綴りでも同じ判定になる。
///
/// 区切りで割ってから見るので、`mytarget/debug/` のような紛れ込みは拾わない。
fn has_target_profile_dir(path: &str, profile: &str) -> bool {
    let segments: Vec<&str> = path.split(['/', '\\']).collect();
    segments
        .windows(2)
        .any(|pair| pair[0] == "target" && pair[1] == profile)
}

#[cfg(test)]
mod tests;
