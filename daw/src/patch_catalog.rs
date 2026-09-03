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
use cmrt_patches::{DrumPatchRole, PatchRole, PatchRoleIndex, PatchRoleInput};
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

/// 用途分類を引く索引の出どころ。
///
/// snapshot が `Ready` ならそれを借り、`Loading` / `Err` のときだけ走査結果から
/// 同じ規則で組み直す。どちらを通っても分類規則は 1 つ。
enum RoleIndexSource {
    Snapshot(Arc<PatchCatalogSnapshot>),
    /// 索引そのものが大きいので box へ逃がす（snapshot 側との差が 300 バイト超）。
    Scanned(Box<PatchRoleIndex>),
}

impl RoleIndexSource {
    fn index(&self) -> &PatchRoleIndex {
        match self {
            Self::Snapshot(snapshot) => snapshot.patch_roles(),
            Self::Scanned(index) => index,
        }
    }
}

/// 用途分類を引く 1 回ぶんの材料（索引 + 表示名一覧）。
///
/// 表示名一覧も持つのは、init セルに短縮名で保存された音色を索引の表示名へ
/// 突き合わせるため（[`Self::resolved_display`]）。
pub(crate) struct PatchRoleLookup {
    pairs: Vec<(String, String)>,
    source: RoleIndexSource,
}

impl PatchRoleLookup {
    fn index(&self) -> &PatchRoleIndex {
        self.source.index()
    }

    /// 保存された patch 名を、索引が知っている表示名へ直す。
    ///
    /// 素通しで当たるならそのまま返し、当たらないときだけ突き合わせる
    /// （当たる場合の線形走査を避ける）。`ui::patch_display::role_of_patch` と同じ手順。
    fn resolved_display(&self, patch_name: &str) -> Option<String> {
        if self.index().role_of(patch_name).is_some() {
            return Some(patch_name.to_string());
        }
        cmrt_patches::resolve_display_patch_name(&self.pairs, patch_name)
    }

    /// `patch_name` と同じ用途の抽選候補と、その抽選デッキのキー。
    ///
    /// catalog がその音色を知らなければ `None`。呼び出し側はそこだけ
    /// 「用途を問わない抽選」へ落とす。
    ///
    /// **drum は部位まで揃える。** role は同じ `drum` でも、kick の track で snare が
    /// 出てきたら別の楽器へ化けたのと変わらない。部位語が当たらない drum
    /// （`drums` だけの音色など）は部位候補が空になるので、role 全体へ広げる。
    fn same_role_candidates(&self, patch_name: &str) -> Option<(String, Vec<String>)> {
        let display = self.resolved_display(patch_name)?;
        let index = self.index();
        let role = index.role_of(&display)?;
        if let Some(drum_role) = index.drum_role_of(&display) {
            let candidates = index.drum_candidates(drum_role);
            if !candidates.is_empty() {
                return Some((drum_deck_key(drum_role), candidates.to_vec()));
            }
        }
        Some((role_deck_key(role), index.candidates(role).to_vec()))
    }
}

fn role_deck_key(role: PatchRole) -> String {
    format!("role:{}", role.key())
}

fn drum_deck_key(role: DrumPatchRole) -> String {
    format!("drum:{}", role.key())
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

    /// catalog 構築時の2回目のload実測を、次回load時間の予想として返す。
    pub(crate) fn catalog_patch_load_estimate_ms(&self, patch_name: &str) -> Option<u64> {
        self.catalog_snapshot()?
            .load_measurements()
            .get(patch_name)?
            .second_load_ms
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

    /// 用途分類を引く材料を、snapshot 優先・走査フォールバックで揃える。
    pub(crate) fn patch_role_lookup(&self, reason: &str) -> Option<PatchRoleLookup> {
        let snapshot = self.catalog_snapshot();
        let pairs = self.patch_pairs_for_lookup(reason)?;
        let source = match snapshot {
            Some(snapshot) => RoleIndexSource::Snapshot(snapshot),
            None => {
                let presets = cmrt_history::load_mml_patch_filter_presets();
                RoleIndexSource::Scanned(Box::new(PatchRoleIndex::build(
                    pairs
                        .iter()
                        .map(|(display, normalized_display)| PatchRoleInput {
                            display,
                            normalized_display,
                            selector_category: None,
                        }),
                    &presets,
                )))
            }
        };
        Some(PatchRoleLookup { pairs, source })
    }

    /// 共通の用途分類から、指定した Role の音色だけを抽選する。
    ///
    /// snapshot が Ready なら selector category とユーザー preset を反映済みの索引を使う。
    /// 読み込み中の走査フォールバックでも同じ分類規則を適用し、別 Role の音色へは
    /// フォールバックしない。
    pub(crate) fn pick_random_patch_name_for_role(&mut self, role: PatchRole) -> Option<String> {
        let lookup = self.patch_role_lookup(&format!("random-patch-role-{}", role.key()))?;
        let candidates = lookup.index().candidates(role).to_vec();
        let deck_key = role_deck_key(role);
        self.draw_from_patch_deck(&deck_key, &candidates)
    }

    /// いま鳴っている音色と同じ用途の音色だけを抽選する。
    ///
    /// catalog がその音色を知らないとき（分類できないとき）だけ `None`。
    /// 呼び出し側はそこだけ「用途を問わない抽選」へ落とす。
    pub(crate) fn pick_random_patch_name_for_same_role_as(
        &mut self,
        patch_name: &str,
    ) -> Option<String> {
        let lookup = self.patch_role_lookup("random-patch-same-role")?;
        let (deck_key, candidates) = lookup.same_role_candidates(patch_name)?;
        self.draw_from_patch_deck(&deck_key, &candidates)
    }

    fn draw_from_patch_deck(&mut self, deck_key: &str, candidates: &[String]) -> Option<String> {
        let idx = self
            .random_patch_decks
            .next_index(Some(deck_key), candidates.len())?;
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
