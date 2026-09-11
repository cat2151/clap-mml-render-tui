//! テスト用の一時ディレクトリを、**プロセスをまたいでも衝突しない**名前で作るヘルパ。
//!
//! `std::env::temp_dir()` へ固定文字列を `join` して直に書くと、同じ workspace で
//! `cargo test` を 2 つ走らせた（人間 + agent、agent 2 匹）ときに
//! **1 つのディレクトリを取り合って** flaky になる。1 プロセスの中では起きないので
//! 単独で何回回しても再現しない。ここを通せば `{pid}_{nanos}` が必ず入る。
//!
//! ## 置き場所について（設計ではなく経緯）
//!
//! このヘルパは undo/redo とは何の関係も無いのに `cmrt-history` に住んでいる。
//! 最初の実装が `daw` にあり、横展開のときにいちばん多くの crate から見える場所が
//! ここだった、というだけの理由。**あるべき姿ではない。**
//!
//! 実害が2つある:
//!
//! - 使いたいだけの crate が `cmrt-history` 一式（`realtime-play` / `patches` /
//!   `tui-core` / `serde` …）を dev-dependency で引き込むことになる
//! - `cmrt-tui-core` と `cmrt-core`(core-lib) からは**構造的に使えない**。
//!   `cmrt-history` がそれらに依存しているので、逆向きの依存は循環する
//!
//! なので低レイヤの crate が同じものを必要とした時点で、依存ゼロの leaf crate
//! （`cmrt-test-support` のような）へ切り出すこと。移動自体は機械的だが、
//! Cargo.toml が6 crate 分動くので「必要になってから」でよい。

use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(test)]
mod tests;

/// 一時ディレクトリ名の共通接頭辞。掃除の対象を見分けるのにも使う。
const TEST_DIR_PREFIX: &str = "cmrt_test_";

/// この時間より古い `cmrt_test_*` は、次のテストプロセスが掃除してよい。
/// テストプロセスは数秒で終わるので、生きているものを巻き込む心配は無い。
const STALE_AGE: Duration = Duration::from_secs(60 * 60);

/// 「末尾の数字は UNIX epoch からの nanos か」の下限（2020-09 ごろ）。
///
/// **これが無いと、末尾が連番のテストディレクトリ
/// （`cmrt_test_notepad_history_enter_flush_{pid}_{連番}`）を「1970 年に作られた」と
/// みなして、走っている最中に消してしまう**（実際に `cmrt-notepad` の
/// `handle_notepad_history_enter_flushes_store` を 1 本落とした）。
const MIN_PLAUSIBLE_NANOS: u128 = 1_600_000_000_000_000_000;

/// 1 プロセスが 1 度に消す上限。**溜まった分を一気に消しに行かせない**ための蓋。
/// 掃除を入れる前の実測で `%TEMP%` に 15313 件たまっていたので、
/// 上限が無いと最初の 1 プロセスがその全部を消すまで戻ってこない。
const MAX_SWEEP_PER_PROCESS: usize = 200;

fn nanos_now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch in tests")
        .as_nanos()
}

/// `%TEMP%/cmrt_test_{tag}_{pid}_{nanos}`。**作成はしない**（パスを返すだけ）。
///
/// 「存在しないディレクトリ」を指したいテストにもそのまま使える
/// （ユニークなので、そこに何かが在ることは無い）。
pub fn unique_test_dir(tag: &str) -> PathBuf {
    sweep_stale_test_dirs_once();
    std::env::temp_dir().join(format!(
        "{TEST_DIR_PREFIX}{tag}_{}_{}",
        std::process::id(),
        nanos_now()
    ))
}

/// drop で中身ごと消える一時ディレクトリ。
///
/// `Deref<Target = Path>` と `AsRef<Path>` を持つので、`&tmp` / `tmp.join("Pads")` /
/// `tmp.display()` はそのまま書ける。`PathBuf` が欲しいときは `to_path_buf()`。
pub struct TempDirGuard(PathBuf);

impl TempDirGuard {
    /// `tag` は「何のテストか」が分かる短い名前。pid と nanos はこの中で付ける。
    pub fn new(tag: &str) -> Self {
        let path = unique_test_dir(tag);
        std::fs::remove_dir_all(&path).ok();
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl std::ops::Deref for TempDirGuard {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for TempDirGuard {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// [`temp_local_dirs`] が返す guard 一式。
pub type LocalDirGuards = (TempDirGuard, super::TestEnvGuard);

/// 実 `%LOCALAPPDATA%` を触らせないための「一時ディレクトリ＋env guard」。
///
/// history / config / cache の出力先をまとめて temp へ逃がす。
/// 戻り値は**両方とも生かしておくこと**（drop で env が戻り、temp が消える）。
///
/// **2 つの落とし穴（どちらも実際に踏んだ）:**
///
/// 1. **ヘルパ関数の中で握って捨てない。** 捨てるとその関数を抜けた時点で env が
///    戻り、呼び出し側は guard 無しで走る（見た目は guard 付きなのに効いていない）。
/// 2. **1 つのテストで 2 回握らない。** [`super::env_lock`] は再入不可なのでデッドロックする。
///    **1 テスト = 1 guard、置き場所はテスト関数の先頭**にすること。
pub fn temp_local_dirs(tag: &str) -> LocalDirGuards {
    let temp = TempDirGuard::new(tag);
    let env_guard = super::set_local_dir_envs(temp.path());
    (temp, env_guard)
}

/// 名前に埋めた nanos から「もう誰も使っていない残骸か」を判定する。
///
/// `cmrt_test_{...}_{pid}_{nanos}` の末尾要素だけを見る。数字で終わらないもの
/// （固定名の残骸や無関係なファイル）は**触らない**。
fn is_stale_test_dir(name: &str, now_nanos: u128, max_age: Duration) -> bool {
    let Some(rest) = name.strip_prefix(TEST_DIR_PREFIX) else {
        return false;
    };
    let Some(created) = rest.rsplit('_').next().and_then(|n| n.parse::<u128>().ok()) else {
        return false;
    };
    // 連番で終わる名前（他 crate のテストが使っている）は nanos ではない。触らない。
    if created < MIN_PLAUSIBLE_NANOS {
        return false;
    }
    now_nanos.saturating_sub(created) > max_age.as_nanos()
}

/// `%TEMP%` にたまった古い `cmrt_test_*` を掃除する（プロセスに 1 度だけ）。
///
/// `default_test_app_dir_path()` が作る `cmrt_test_process_*` は**誰も消さない**ので、
/// 放っておくと数千件たまる（実測 3654 件）。`%TEMP%` の走査が遅くなるだけとはいえ、
/// 掃除の当てが他に無いのでテスト側で拾う。
pub(super) fn sweep_stale_test_dirs_once() {
    static SWEPT: std::sync::Once = std::sync::Once::new();
    SWEPT.call_once(|| {
        let now = nanos_now();
        let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
            return;
        };
        let mut removed = 0usize;
        for entry in entries.flatten() {
            if removed >= MAX_SWEEP_PER_PROCESS {
                break;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !is_stale_test_dir(name, now, STALE_AGE) {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path).ok();
            } else {
                std::fs::remove_file(&path).ok();
            }
            removed += 1;
        }
    });
}
