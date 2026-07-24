//! テストユーティリティ。
//!
//! history / config / ログのパスをテスト専用ディレクトリへ差し替えるヘルパは、
//! 永続化層と同じ場所にある方が壊れにくいため `cmrt-history` の `test_support` へ
//! 切り出した。従来の `crate::test_utils::*` パスは再エクスポートで維持する。
//! ratatui バッファを読む UI テストヘルパは、DAW crate 等とも共有するため
//! `cmrt-tui-core` の `buffer_test`（test-support feature）へ切り出した。

use std::path::Path;

pub(crate) use cmrt_history::test_support::{
    session_state_path_for_test, set_local_dir_envs, test_app_dir_for_current_thread_or_default,
    TestEnvGuard,
};
pub(crate) use cmrt_tui_core::buffer_test::{find_text_ignoring_spaces, help_overlay_bounds};

/// 旧ヘルパ名を使っているテスト向けの後方互換ラッパ。
#[allow(dead_code)]
pub(crate) fn set_data_local_dir_envs(base: &Path) -> TestEnvGuard {
    set_local_dir_envs(base)
}
