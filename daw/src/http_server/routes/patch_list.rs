//! `GET /patches` が返す音色名一覧。
//!
//! ハンドラから切り出してあるのは、`tiny_http::Request` を組み立てずに
//! ヘッドレスで検証できるようにするため。

use std::sync::{Arc, Mutex};

use cmrt_runtime::Config;
use cmrt_tui_core::patch_load::PatchLoadState;

use crate::patch_catalog::{lookup_patch_pairs, snapshot_patch_pairs, PatchScanPolicy};

/// 音色名一覧を、注入 snapshot 優先・走査フォールバックで作る。
///
/// `patch_load` が `None`（＝ DAW 起動前に立った古い state）でも走査へ落ちるだけで動く。
pub(in crate::http_server) fn http_patch_names(
    cfg: &Config,
    patch_load: Option<&Arc<Mutex<PatchLoadState>>>,
) -> Result<Vec<String>, (u16, String)> {
    let snapshot = patch_load.and_then(|patch_load| snapshot_patch_pairs(patch_load));
    let lookup = lookup_patch_pairs(
        snapshot,
        cfg,
        PatchScanPolicy::OnlyWhenPatchDirsConfigured,
        "http-get-patches",
    );
    crate::log_line(&lookup.log_line);
    match lookup.pairs {
        Some(pairs) => Ok(pairs
            .into_iter()
            .map(|(patch_name, _)| patch_name)
            .collect()),
        None => Err((500, "patch 一覧の取得に失敗しました\n".to_string())),
    }
}

#[cfg(test)]
mod tests;
