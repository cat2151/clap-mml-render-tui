//! DAW から見た patch catalog（音色一覧）の入口。
//!
//! 一覧の取得口をこの 1 か所へ集める。
//!
//! app が起動時に立てた file cache 由来の `PatchLoadState` を注入で受け取り、
//! `Ready` ならその snapshot から一覧を借りる。DAW だけが
//! `collect_patch_pairs()`（= 実 file の全走査 + `canonicalize`）を直に呼んでいて、
//! 実測で 5120 patch あたり **1.3 秒** UI をブロックしていた。file cache 経路なら
//! 同じ内容が 80ms 台で載る（起動時に 1 回だけ）。
//!
//! 走査経路は snapshot がまだ `Loading` / `Err` のときのフォールバックとしてだけ残す。

use super::DawApp;
use cmrt_patches::{PatchRole, PatchRoleIndex, PatchRoleInput};
use cmrt_runtime::Config;
use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadState};
use std::sync::{Arc, Mutex};

/// どちらの経路で patch 一覧を得たか。ログの `source=` に出す。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PatchPairsSource {
    /// app から注入された file cache の snapshot。
    Snapshot,
    /// 実 file の走査（snapshot が未 Ready のときだけ）。
    Scan,
}

impl PatchPairsSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
            Self::Scan => "scan",
        }
    }
}

/// snapshot が無いときに走査してよいか。
///
/// **`patches_dirs` が未設定でも走査は 0 件では終わらない。** `catalog_plugins()` は
/// `patches_dirs` とは独立に「この開発機にインストール済みのプラグイン」の音色置き場も
/// カタログへ載せるので、未設定のまま走査すると Dexed / Floe などの音色が数千件返る。
/// 元々 `has_configured_patch_dirs()` で門番していた呼び出し口は、snapshot が無いときも
/// その門番を保つ必要がある。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PatchScanPolicy {
    /// snapshot が無ければ常に走査する。
    Always,
    /// snapshot が無いときは `patches_dirs` が設定されているときだけ走査する。
    /// 未設定なら走査せず **0 件**を返す（「取得に失敗」とは区別する）。
    OnlyWhenPatchDirsConfigured,
}

/// patch 一覧の取得 1 回ぶんの結果。ログ 1 行の組み立てまでここで済ませる。
pub(crate) struct PatchPairsLookup {
    /// 取得できなかったとき（走査エラー）だけ `None`。
    pub(crate) pairs: Option<Vec<(String, String)>>,
    /// 呼び出し口が画面ログ / グローバルログのどちらへ流すかは呼び出し側で決める。
    pub(crate) log_line: String,
}

/// 注入された `PatchLoadState` が `Ready` なら patch 一覧を借りる。
pub(crate) fn snapshot_patch_pairs(
    patch_load: &Mutex<PatchLoadState>,
) -> Option<Vec<(String, String)>> {
    match &*patch_load.lock().unwrap() {
        PatchLoadState::Ready(snapshot) => Some(snapshot.pairs().to_vec()),
        PatchLoadState::Loading | PatchLoadState::Err(_) => None,
    }
}

/// patch 一覧を snapshot 優先・走査フォールバックで取得する、DAW 全体で 1 つの実装。
///
/// `DawApp` を持たない HTTP ハンドラからも呼べるよう、`self` を取らない自由関数にしてある。
pub(crate) fn lookup_patch_pairs(
    snapshot_pairs: Option<Vec<(String, String)>>,
    cfg: &Config,
    policy: PatchScanPolicy,
    reason: &str,
) -> PatchPairsLookup {
    let started = std::time::Instant::now();
    let (source, pairs) = match snapshot_pairs {
        Some(pairs) => (PatchPairsSource::Snapshot, Some(pairs)),
        None => (PatchPairsSource::Scan, scan_patch_pairs(cfg, policy)),
    };
    let log_line = format!(
        "daw: event=patch-pairs source={} count={} ms={} reason={reason}",
        source.as_str(),
        pairs.as_ref().map_or(0, Vec::len),
        started.elapsed().as_millis(),
    );
    PatchPairsLookup { pairs, log_line }
}

fn scan_patch_pairs(cfg: &Config, policy: PatchScanPolicy) -> Option<Vec<(String, String)>> {
    if policy == PatchScanPolicy::OnlyWhenPatchDirsConfigured
        && !cmrt_tui_core::patches::has_configured_patch_dirs(cfg)
    {
        return Some(Vec::new());
    }
    cmrt_tui_core::patches::collect_patch_pairs(cfg).ok()
}

/// 注入された `PatchLoadState` が `Ready` なら snapshot そのものを借りる。
///
/// `pairs()` だけでなく `patch_roles()` も要る呼び出し口（grid の init 列表示）向け。
/// 1 描画につき 1 回だけ lock して `Arc` を持ち出し、セルごとに lock し直さないこと。
pub(crate) fn snapshot(patch_load: &Mutex<PatchLoadState>) -> Option<Arc<PatchCatalogSnapshot>> {
    match &*patch_load.lock().unwrap() {
        PatchLoadState::Ready(snapshot) => Some(Arc::clone(snapshot)),
        PatchLoadState::Loading | PatchLoadState::Err(_) => None,
    }
}

impl DawApp {
    /// 注入された snapshot を借りる。`Loading` / `Err` なら `None`。
    pub(crate) fn catalog_snapshot(&self) -> Option<Arc<PatchCatalogSnapshot>> {
        snapshot(&self.patch_load)
    }

    /// 注入された snapshot が `Ready` なら patch 一覧を返す。`Loading` / `Err` なら `None`。
    pub(crate) fn catalog_patch_pairs(&self) -> Option<Vec<(String, String)>> {
        snapshot_patch_pairs(&self.patch_load)
    }

    /// patch 一覧を、snapshot 優先・走査フォールバックで取得してログを 1 行残す。
    ///
    /// どちらを通ったかを必ず出すのは、遅延の再発を「体感」ではなくログで切り分けるため。
    pub(crate) fn patch_pairs_for_lookup(&self, reason: &str) -> Option<Vec<(String, String)>> {
        self.patch_pairs_for_lookup_with_policy(reason, PatchScanPolicy::Always)
    }

    /// `has_configured_patch_dirs()` で門番していた呼び出し口用。
    /// snapshot が無く `patches_dirs` も未設定なら、走査せず 0 件を返す。
    pub(crate) fn patch_pairs_for_configured_dirs(
        &self,
        reason: &str,
    ) -> Option<Vec<(String, String)>> {
        self.patch_pairs_for_lookup_with_policy(
            reason,
            PatchScanPolicy::OnlyWhenPatchDirsConfigured,
        )
    }

    fn patch_pairs_for_lookup_with_policy(
        &self,
        reason: &str,
        policy: PatchScanPolicy,
    ) -> Option<Vec<(String, String)>> {
        let PatchPairsLookup { pairs, log_line } = lookup_patch_pairs(
            self.catalog_patch_pairs(),
            self.cfg.as_ref(),
            policy,
            reason,
        );
        self.append_log_line(log_line);
        pairs
    }

    // ─── ランダム音色 ─────────────────────────────────────────

    /// Notepad と同じく、category、vendor、filename を含む表示パス全文を検索する。
    /// grid sequencer の filename stem 専用検索とは検索範囲が異なる。
    fn patch_display_path_query_terms(query: Option<&str>) -> Option<Vec<String>> {
        query
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .map(|query| {
                query
                    .split_whitespace()
                    .map(|term| term.to_lowercase())
                    .collect()
            })
    }

    fn patch_display_path_matches_query(lower_display_path: &str, terms: &[String]) -> bool {
        terms
            .iter()
            .all(|term| lower_display_path.contains(term.as_str()))
    }

    fn filter_patch_pairs_by_display_path_query(
        patches: Vec<(String, String)>,
        query: Option<&str>,
    ) -> Vec<(String, String)> {
        let Some(terms) = Self::patch_display_path_query_terms(query) else {
            return patches;
        };
        patches
            .into_iter()
            .filter(|(_, lower)| Self::patch_display_path_matches_query(lower, &terms))
            .collect()
    }

    pub(crate) fn filter_patch_names_by_display_path_query(
        all: &[(String, String)],
        query: &str,
    ) -> Vec<String> {
        let Some(terms) = Self::patch_display_path_query_terms(Some(query)) else {
            return all.iter().map(|(orig, _)| orig.clone()).collect();
        };
        all.iter()
            .filter(|(_, lower)| Self::patch_display_path_matches_query(lower, &terms))
            .map(|(orig, _)| orig.clone())
            .collect()
    }

    pub(crate) fn pick_random_patch_name(&mut self) -> Option<String> {
        self.pick_random_patch_name_with_query(None)
    }

    /// 共通の用途分類から、指定した Role の音色だけを抽選する。
    ///
    /// snapshot が Ready なら selector category とユーザー preset を反映済みの索引を使う。
    /// 読み込み中の走査フォールバックでも同じ分類規則を適用し、別 Role の音色へは
    /// フォールバックしない。
    pub(crate) fn pick_random_patch_name_for_role(&mut self, role: PatchRole) -> Option<String> {
        let snapshot = self.catalog_snapshot();
        let patches = self.patch_pairs_for_lookup(&format!("random-patch-role-{}", role.key()))?;
        let candidates = match snapshot {
            Some(snapshot) => snapshot.patch_roles().candidates(role).to_vec(),
            None => {
                let presets = cmrt_history::load_mml_patch_filter_presets();
                PatchRoleIndex::build(
                    patches
                        .iter()
                        .map(|(display, normalized_display)| PatchRoleInput {
                            display,
                            normalized_display,
                            selector_category: None,
                        }),
                    &presets,
                )
                .candidates(role)
                .to_vec()
            }
        };
        let deck_key = format!("role:{}", role.key());
        let idx = self
            .random_patch_decks
            .next_index(Some(&deck_key), candidates.len())?;
        Some(candidates[idx].clone())
    }

    pub(crate) fn pick_random_patch_name_with_query(
        &mut self,
        query: Option<&str>,
    ) -> Option<String> {
        let patches = self.patch_pairs_for_lookup("random-patch")?;
        let candidates = Self::filter_patch_pairs_by_display_path_query(patches, query)
            .into_iter()
            .map(|(orig, _)| orig)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        let idx = self
            .random_patch_decks
            .next_index(query, candidates.len())?;
        Some(candidates[idx].clone())
    }
}

#[cfg(test)]
mod tests;
