//! 実体の決め方と profile 判定のテスト。
//!
//! パス解決のテストは実ファイルを temp に作る。開発機に何が入っているかで
//! 結果が変わると、事故の再発防止という目的そのものが果たせない。

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use super::*;

/// temp に「repo が 2 本並んだ形」を作る。
///
/// ```text
/// <root>/clap-mml-render-tui/target/<profile>/cmrt.exe
/// <root>/clap-mml-play-server/target/release/clap-mml-realtime-play-server.exe
/// ```
struct TwoRepos {
    root: PathBuf,
}

impl TwoRepos {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "cmrt_server_binary_{name}_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    /// `cmrt.exe` が置かれるパスを返す。実体は作らない（存在判定には使わないため）。
    fn cmrt_exe(&self, profile: &str) -> PathBuf {
        let dir = self
            .root
            .join("clap-mml-render-tui")
            .join("target")
            .join(profile);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("cmrt.exe")
    }

    fn create_server_next_to(&self, cmrt_exe: &Path) -> PathBuf {
        create_file(&cmrt_exe.with_file_name(default_realtime_play_server_executable_name()))
    }

    /// 兄弟 repo の指定 profile に実体を作る。
    fn create_play_server_repo_build(&self, profile: &str) -> PathBuf {
        create_file(
            &self
                .root
                .join("clap-mml-play-server")
                .join("target")
                .join(profile)
                .join(default_realtime_play_server_executable_name()),
        )
    }
}

impl Drop for TwoRepos {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn create_file(path: &Path) -> PathBuf {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, []).unwrap();
    path.to_path_buf()
}

fn resolved(binary: &ServerBinary) -> &ResolvedServer {
    match binary {
        ServerBinary::Resolved(resolved) => resolved,
        ServerBinary::NotFound { searched } => {
            panic!("実体が見つかること。探した場所: {searched:?}")
        }
    }
}

#[test]
fn an_explicit_argument_wins_over_everything_else() {
    let repos = TwoRepos::new("argument");
    let cmrt = repos.cmrt_exe("debug");
    // 2 番でも 3 番でも解決できる状態にしておく。それでも 1 番が勝つ。
    repos.create_server_next_to(&cmrt);
    repos.create_play_server_repo_build("release");
    let explicit = create_file(&repos.root.join("elsewhere").join("server.exe"));

    let binary = resolve_with(
        Some(&PlayServerLaunch::Executable(explicit.clone())),
        Some(&cmrt),
    );

    assert_eq!(resolved(&binary).source, ServerSource::Argument);
    assert_eq!(resolved(&binary).exe, explicit.display().to_string());
}

/// 打った指定が黙って無視されるのは、この ADR が潰した事故と同じ手触りになる。
#[test]
fn a_missing_argument_does_not_fall_back_to_the_search() {
    let repos = TwoRepos::new("argument-missing");
    let cmrt = repos.cmrt_exe("release");
    repos.create_server_next_to(&cmrt);

    let missing = repos.root.join("no-such-server.exe");
    let binary = resolve_with(Some(&PlayServerLaunch::Executable(missing)), Some(&cmrt));

    let ServerBinary::NotFound { searched } = binary else {
        panic!("探索へ落とさずエラーにすること: {binary:?}");
    };
    assert_eq!(searched.len(), 1, "探した場所は指定された 1 か所だけ");
    assert!(searched[0].contains("--play-server"), "{searched:?}");
}

#[test]
fn the_executable_next_to_cmrt_is_used_when_there_is_no_argument() {
    let repos = TwoRepos::new("sibling");
    let cmrt = repos.cmrt_exe("release");
    let sibling = repos.create_server_next_to(&cmrt);
    repos.create_play_server_repo_build("release");

    let binary = resolve_with(None, Some(&cmrt));

    assert_eq!(resolved(&binary).source, ServerSource::SiblingDirectory);
    assert_eq!(resolved(&binary).exe, sibling.display().to_string());
}

#[test]
fn the_play_server_repo_release_is_used_when_nothing_sits_next_to_cmrt() {
    let repos = TwoRepos::new("repo-release");
    let cmrt = repos.cmrt_exe("debug");
    let repo_release = repos.create_play_server_repo_build("release");

    let binary = resolve_with(None, Some(&cmrt));

    assert_eq!(
        resolved(&binary).source,
        ServerSource::PlayServerRepoRelease
    );
    assert_eq!(resolved(&binary).exe, repo_release.display().to_string());
}

/// 今回の事故そのもの。起動用 bat が PATH の先頭へ載せていたのは
/// **兄弟 repo の `target/debug`** で、そこにしか実体が無くても選ばれてはいけない。
/// 見つからないなら、探した場所を言って止まる。
#[test]
fn the_debug_build_that_the_old_path_switch_pointed_at_is_never_chosen() {
    let repos = TwoRepos::new("no-path");
    let cmrt = repos.cmrt_exe("debug");
    repos.create_play_server_repo_build("debug");

    let binary = resolve_with(None, Some(&cmrt));

    let ServerBinary::NotFound { searched } = binary else {
        panic!("debug のサーバーを選ばないこと: {binary:?}");
    };
    assert_eq!(searched.len(), 2, "探した 2 か所を言うこと: {searched:?}");
    assert!(
        searched.iter().any(|place| place.contains("cmrt")),
        "{searched:?}"
    );
    assert!(
        searched.iter().any(|place| place.contains("release")),
        "{searched:?}"
    );
}

#[test]
fn not_found_lines_say_where_it_looked() {
    let lines = ServerBinary::not_found_lines(&["どこか".to_owned()]);

    assert_eq!(
        lines[0],
        format!(
            "play server の実体が見つかりません（{}）",
            default_realtime_play_server_executable_name()
        )
    );
    assert_eq!(lines[1], "探した場所: どこか");
}

/// 配布物では `cmrt.exe` が `target/` 配下に居ないので、3 番は成立しない。
/// ここが成立すると、ユーザーの PC のどこか上位に同名フォルダがあるだけで
/// 知らない実体を掴むことになる。
#[test]
fn the_repo_release_path_only_applies_to_a_cargo_build_layout() {
    let repos = TwoRepos::new("installed");
    let installed = repos
        .root
        .join("Program Files")
        .join("cmrt")
        .join("cmrt.exe");
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    repos.create_play_server_repo_build("release");

    let binary = resolve_with(None, Some(&installed));

    let ServerBinary::NotFound { searched } = binary else {
        panic!("配布物の配置で兄弟 repo を掴まないこと: {binary:?}");
    };
    assert_eq!(
        searched.len(),
        1,
        "探すのは同じディレクトリだけ: {searched:?}"
    );
}

#[test]
fn the_profile_is_read_from_the_path_in_both_spellings() {
    for path in [
        r"N:\projects\clap-mml-play-server\target\debug\server.exe",
        "/home/x/clap-mml-play-server/target/debug/server",
    ] {
        assert_eq!(
            classify(path, ServerSource::Argument),
            ServerProfile::Debug,
            "{path}"
        );
    }
    for path in [
        r"N:\projects\clap-mml-play-server\target\release\server.exe",
        "/home/x/clap-mml-play-server/target/release/server",
    ] {
        assert_eq!(
            classify(path, ServerSource::Argument),
            ServerProfile::Release,
            "{path}"
        );
    }
}

/// `./target/release` へ手で cp するのは禁止だが、やってしまったときに
/// 「同梱」と名乗って静かになるのが一番まずい。パスの判定を先に見る理由。
#[test]
fn a_binary_copied_into_target_debug_is_still_reported_as_debug() {
    let path = r"N:\projects\clap-mml-render-tui\target\debug\clap-mml-realtime-play-server.exe";

    assert_eq!(
        classify(path, ServerSource::SiblingDirectory),
        ServerProfile::Debug
    );
}

#[test]
fn a_bundled_binary_is_not_highlighted() {
    let path = r"C:\Users\x\cmrt\clap-mml-realtime-play-server.exe";

    let profile = classify(path, ServerSource::SiblingDirectory);

    assert_eq!(profile, ServerProfile::Bundled);
    assert!(
        !profile.needs_attention(),
        "配布物の通常運転で警告を出さないこと"
    );
}

#[test]
fn debug_and_unknown_are_the_ones_that_need_attention() {
    assert!(ServerProfile::Debug.needs_attention());
    assert!(ServerProfile::Unknown.needs_attention());
    assert!(!ServerProfile::Release.needs_attention());
    assert!(!ServerProfile::Bundled.needs_attention());
}

/// `mytarget/debug/` のような紛れ込みを拾わない。
#[test]
fn a_directory_that_merely_ends_with_target_is_not_a_cargo_target_dir() {
    assert_eq!(
        classify(
            r"N:\projects\mytarget\debug\server.exe",
            ServerSource::Argument
        ),
        ServerProfile::Unknown
    );
}

/// ファイルの mtime を明示的に置く。
///
/// 作った順に頼ると、同じ tick に落ちたときだけ落ちる flaky になる。
fn set_modified(path: &Path, seconds_from_epoch: u64) {
    let time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds_from_epoch);
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

/// この ADR が PATH 解決を潰した代わりに生まれた穴。
/// 「兄弟 repo を直して debug だけ建て、古い release が動き続ける」を検出する。
#[test]
fn a_binary_older_than_its_sources_is_reported_as_stale() {
    let repos = TwoRepos::new("stale");
    let cmrt = repos.cmrt_exe("release");
    let server = repos.create_play_server_repo_build("release");
    let source = create_file(
        &repos
            .root
            .join("clap-mml-play-server")
            .join("core-lib")
            .join("src")
            .join("lib.rs"),
    );
    set_modified(&server, 1_000);
    set_modified(&source, 1_042);

    let binary = resolve_with(None, Some(&cmrt));

    let stale = resolved(&binary).stale.as_ref().expect("古いと言うこと");
    assert!(stale.newest_source.ends_with("lib.rs"), "{stale:?}");
    assert_eq!(stale.newer_by_seconds, 42);
    assert!(
        resolved(&binary).needs_attention(),
        "release でも画面で目立たせること"
    );
}

#[test]
fn a_binary_newer_than_its_sources_is_not_stale() {
    let repos = TwoRepos::new("fresh");
    let cmrt = repos.cmrt_exe("release");
    let server = repos.create_play_server_repo_build("release");
    let source = create_file(
        &repos
            .root
            .join("clap-mml-play-server")
            .join("core-lib")
            .join("src")
            .join("lib.rs"),
    );
    set_modified(&source, 1_000);
    set_modified(&server, 1_042);

    let binary = resolve_with(None, Some(&cmrt));

    assert_eq!(resolved(&binary).stale, None);
    assert!(
        !resolved(&binary).needs_attention(),
        "通常運転で警告を出さないこと"
    );
}

/// README を直しただけで「古い」と言われると、点きっぱなしの警告になって読まれなくなる。
/// 成果物（`target/`）も見ない。見ると自分自身より新しいファイルが常にある。
#[test]
fn only_rust_and_cargo_files_count_as_sources() {
    let repos = TwoRepos::new("non-source");
    let cmrt = repos.cmrt_exe("release");
    let server = repos.create_play_server_repo_build("release");
    let repo = repos.root.join("clap-mml-play-server");
    let readme = create_file(&repo.join("README.md"));
    let artifact = create_file(&repo.join("target").join("release").join("build.log.rs"));
    set_modified(&server, 1_000);
    set_modified(&readme, 1_042);
    set_modified(&artifact, 1_042);

    let binary = resolve_with(None, Some(&cmrt));

    assert_eq!(resolved(&binary).stale, None);
}

/// 配布物には比べるソースが無い。毎回ファイル走査をしても得るものが無いので判定しない。
#[test]
fn a_bundled_binary_is_never_checked_for_staleness() {
    let repos = TwoRepos::new("bundled-stale");
    let installed = repos.root.join("cmrt").join("cmrt.exe");
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    let server = repos.create_server_next_to(&installed);
    set_modified(&server, 1_000);

    let binary = resolve_with(None, Some(&installed));

    assert_eq!(resolved(&binary).profile, ServerProfile::Bundled);
    assert_eq!(resolved(&binary).stale, None);
}

#[test]
fn the_log_line_says_how_stale_the_binary_is() {
    let resolved = ResolvedServer {
        exe: "/x/target/release/server".to_owned(),
        source: ServerSource::PlayServerRepoRelease,
        profile: ServerProfile::Release,
        stale: Some(StaleSource {
            newest_source: "/x/core-lib/src/lib.rs".to_owned(),
            newer_by_seconds: 42,
        }),
    };

    let fields = resolved.log_fields();

    assert!(fields.contains("stale_by_s=42"), "{fields}");
    assert!(fields.contains("lib.rs"), "{fields}");
}
