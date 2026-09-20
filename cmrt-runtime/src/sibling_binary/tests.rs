//! 探索そのもののテスト。実ファイルを temp に作る。開発機に何が入っているかで
//! 結果が変わると、事故の再発防止という目的そのものが果たせない。
//!
//! render-server が実際に探す組み合わせ（`cmrt.exe` → `clap-mml-render-server.exe`）で書く。
//! play server 側（`clap-mml-realtime-play-server.exe`）の同じ探索は
//! `cmrt-realtime-play` の `server_binary/tests.rs` がこの関数を通して確かめる。

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use super::*;

const RENDER_SERVER_EXE_NAME: &str = "clap-mml-render-server.exe";

/// temp に「repo が 2 本並んだ形」を作る。
///
/// ```text
/// <root>/clap-mml-render-tui/target/<profile>/cmrt.exe
/// <root>/clap-mml-play-server/target/release/clap-mml-render-server.exe
/// ```
struct TwoRepos {
    root: PathBuf,
}

impl TwoRepos {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "cmrt_sibling_binary_{name}_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn cmrt_exe(&self, profile: &str) -> PathBuf {
        let dir = self
            .root
            .join("clap-mml-render-tui")
            .join("target")
            .join(profile);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("cmrt.exe")
    }

    fn create_render_server_next_to(&self, cmrt_exe: &Path) -> PathBuf {
        create_file(&cmrt_exe.with_file_name(RENDER_SERVER_EXE_NAME))
    }

    fn create_render_server_repo_build(&self, profile: &str) -> PathBuf {
        create_file(
            &self
                .root
                .join("clap-mml-play-server")
                .join("target")
                .join(profile)
                .join(RENDER_SERVER_EXE_NAME),
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

#[test]
fn the_executable_next_to_current_exe_is_used_when_it_exists() {
    let repos = TwoRepos::new("sibling");
    let cmrt = repos.cmrt_exe("release");
    let sibling = repos.create_render_server_next_to(&cmrt);
    repos.create_render_server_repo_build("release");

    let resolved = resolve_sibling_binary(
        Some(&cmrt),
        RENDER_SERVER_EXE_NAME,
        PLAY_SERVER_REPO_DIR_NAME,
    )
    .unwrap();

    assert_eq!(resolved.source, SiblingBinarySource::SiblingDirectory);
    assert_eq!(resolved.path, sibling);
}

/// 「`<root>/clap-mml-render-tui/target/release/cmrt.exe` から
/// `<root>/clap-mml-play-server/target/release/clap-mml-render-server.exe` を選ぶ」。
#[test]
fn the_sibling_repo_release_is_used_when_nothing_sits_next_to_current_exe() {
    let repos = TwoRepos::new("repo-release");
    let cmrt = repos.cmrt_exe("release");
    let repo_release = repos.create_render_server_repo_build("release");

    let resolved = resolve_sibling_binary(
        Some(&cmrt),
        RENDER_SERVER_EXE_NAME,
        PLAY_SERVER_REPO_DIR_NAME,
    )
    .unwrap();

    assert_eq!(resolved.source, SiblingBinarySource::SiblingRepoRelease);
    assert_eq!(resolved.path, repo_release);
}

/// 兄弟 repo に debug しか無くても選ばれない。「両方無ければ `NotFound` に 2 つの探した場所が並ぶ」。
#[test]
fn neither_location_existing_reports_both_searched_places() {
    let repos = TwoRepos::new("not-found");
    let cmrt = repos.cmrt_exe("debug");
    repos.create_render_server_repo_build("debug");

    let searched = resolve_sibling_binary(
        Some(&cmrt),
        RENDER_SERVER_EXE_NAME,
        PLAY_SERVER_REPO_DIR_NAME,
    )
    .unwrap_err();

    assert_eq!(searched.len(), 2, "{searched:?}");
    assert!(searched.iter().any(|place| place.contains("cmrt")));
    assert!(searched.iter().any(|place| place.contains("release")));
}

/// 配布物では `cmrt.exe` が `target/` 配下に居ないので、兄弟 repo の探索は成立しない。
#[test]
fn the_repo_release_path_only_applies_to_a_cargo_build_layout() {
    let repos = TwoRepos::new("installed");
    let installed = repos
        .root
        .join("Program Files")
        .join("cmrt")
        .join("cmrt.exe");
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    repos.create_render_server_repo_build("release");

    let searched = resolve_sibling_binary(
        Some(&installed),
        RENDER_SERVER_EXE_NAME,
        PLAY_SERVER_REPO_DIR_NAME,
    )
    .unwrap_err();

    assert_eq!(
        searched.len(),
        1,
        "探すのは同じディレクトリだけ: {searched:?}"
    );
}

#[test]
fn no_current_exe_reports_that_its_own_location_was_unavailable() {
    let searched = resolve_sibling_binary(None, RENDER_SERVER_EXE_NAME, PLAY_SERVER_REPO_DIR_NAME)
        .unwrap_err();

    assert_eq!(
        searched,
        vec!["(自分自身の場所が取れませんでした)".to_owned()]
    );
}

#[test]
fn not_found_lines_name_the_entity_and_where_it_looked() {
    let lines = not_found_lines(
        "render-server",
        RENDER_SERVER_EXE_NAME,
        &["どこか".to_owned()],
    );

    assert_eq!(
        lines[0],
        format!("render-server の実体が見つかりません（{RENDER_SERVER_EXE_NAME}）")
    );
    assert_eq!(lines[1], "探した場所: どこか");
}
