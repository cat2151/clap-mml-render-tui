use cmrt_tui_core::patch_load::PatchLoadState;
use cmrt_tui_core::patch_plugins::{CatalogPlugin, PatchPlugins};
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::history::VoicingCache;
use crate::realtime_play::PatchVoicing;
use crate::voicing_sources::{VoicingLayers, VoicingSourceRefresh};

mod vvp_voicings;

pub(in crate::tui) use vvp_voicings::VvpVoicings;

/// patch の mono/poly をどう決めるか。使用中プラグインで切り替わる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum VoicingPolicy {
    /// Surge XT: 共有 JSON・ユーザー判定・override の層から引く。
    /// 未判定の patch は `None`（＝和音向きとはみなさない）。
    Sources,
    /// mono/poly を patch 単位で判定する手段が無いプラグイン: すべて poly とみなす。
    ///
    /// Dexed がこれに当たる。Dexed の mono/poly は `MonoMode` ＝ **instance の設定**
    /// であって cartridge program の属性ではなく、その既定値が POLY であることは
    /// play-server 側の実プラグインテスト
    /// （`dexed_mono_mode_stays_poly_for_every_program`）で固定してある。
    ///
    /// 実行時 probe も使えない。probe は CLAP note event の `note_id` に対する
    /// NOTE_END を数える方式で、note dialect が MIDI のみのプラグインでは
    /// NOTE_END が返らないため「mono」と「測れなかった」を区別できない。
    ///
    /// 判定手段が無いまま `None` を返すと、和音行の候補が必ず 0 件になって
    /// 画面ごと使えなくなる。poly と外した場合の実害（和音行で単音の音色が鳴る）
    /// のほうが軽いので、poly 側へ倒す。
    AssumePoly,
    /// Vaporizer2: 音色ファイル（`.vvp`）の先頭に書いてある `m_uPolyMode` を読む。
    ///
    /// 実行時 probe が使えないのは [`VoicingPolicy::AssumePoly`] と同じ理由（note dialect が
    /// MIDI だけ）だが、こちらは**ファイルに答えが書いてある**ので poly へ倒す必要が無い。
    /// 出荷プリセット 460 件のうち 144 件が Mono で、poly へ倒すと和音行へ出てしまう
    /// （鳴らすと最後の 1 音しか出ない）。
    ///
    /// 読みの実体は [`VvpVoicings`]。
    VvpHeader,
}

impl VoicingPolicy {
    pub(in crate::tui) fn for_plugin(plugin: &CatalogPlugin) -> Self {
        if plugin.is_surge_xt() {
            Self::Sources
        } else if plugin.is_vaporizer2() {
            Self::VvpHeader
        } else {
            Self::AssumePoly
        }
    }

    /// 診断表示（`cmrt patch-roles`）用の説明。
    pub(in crate::tui) fn label(self) -> &'static str {
        match self {
            Self::Sources => "Sources（共有 JSON / ユーザー判定 / override から引く）",
            Self::AssumePoly => "AssumePoly（判定手段が無いので全 patch を poly とみなす）",
            Self::VvpHeader => "VvpHeader（.vvp の m_uPolyMode をファイルの先頭から読む）",
        }
    }
}

/// patch ごとの voicing 判定方針。
///
/// カタログに複数プラグインの音色が並ぶと [`VoicingPolicy`] は 1 つでは足りない。
/// Surge の patch は [`VoicingPolicy::Sources`]、cartridge の patch は
/// [`VoicingPolicy::AssumePoly`] を**同時に**使う必要がある。
pub(in crate::tui) struct VoicingPolicies {
    plugins: PatchPlugins,
    patch_load_state: Option<Arc<Mutex<PatchLoadState>>>,
}

impl VoicingPolicies {
    pub(in crate::tui) fn from_config(cfg: &Config) -> Self {
        Self {
            plugins: fallback_plugins(cfg),
            patch_load_state: None,
        }
    }

    pub(in crate::tui) fn with_patch_load_state(
        cfg: &Config,
        patch_load_state: Arc<Mutex<PatchLoadState>>,
    ) -> Self {
        Self {
            plugins: fallback_plugins(cfg),
            patch_load_state: Some(patch_load_state),
        }
    }

    #[cfg(test)]
    fn from_catalog(plugins: Vec<CatalogPlugin>) -> Self {
        Self {
            plugins: PatchPlugins::from_catalog(plugins),
            patch_load_state: None,
        }
    }

    /// 判定方針と、その判定に使うプラグイン。
    ///
    /// [`VoicingPolicy::VvpHeader`] は display 文字列を実ファイルへ戻すのに基点が要るので、
    /// 方針だけでは足りない。
    fn plugin_for_patch(&self, patch: &str) -> CatalogPlugin {
        if let Some(state) = &self.patch_load_state {
            let state = state.lock().unwrap();
            if let PatchLoadState::Ready(snapshot) = &*state {
                return snapshot.patch_plugins().for_patch(patch).clone();
            }
        }
        self.plugins.for_patch(patch).clone()
    }
}

/// keyboard / grid sequencer の voicing（patch ごとの mono/poly 判定）解決に必要な状態一式。
///
/// 判定結果キャッシュ（`cache`）・解決レイヤ（`layers`）・バックグラウンド更新
/// ハンドル（`source_refresh`）をまとめて保持する。これらは voicing 解決で
/// 常に一緒に使われるため、所有を1型へ集約する。
pub(in crate::tui) struct VoicingState {
    pub(in crate::tui) cache: VoicingCache,
    pub(in crate::tui) layers: VoicingLayers,
    pub(in crate::tui) source_refresh: VoicingSourceRefresh,
    policies: VoicingPolicies,
    /// `.vvp` のmono/poly。TUIではpatch catalog cacheから復元する。
    /// 音色を差し替えた場合は明示的cache再構築で更新する。
    vvp: VvpVoicings,
}

impl VoicingState {
    pub(in crate::tui) fn new(
        cache: VoicingCache,
        layers: VoicingLayers,
        source_refresh: VoicingSourceRefresh,
        policies: VoicingPolicies,
    ) -> Self {
        Self::with_vvp_voicings(
            cache,
            layers,
            source_refresh,
            policies,
            VvpVoicings::default(),
        )
    }

    /// memoを外から渡す形。file cache workerが復元したmemoを共有するために要る。
    pub(in crate::tui) fn with_vvp_voicings(
        cache: VoicingCache,
        layers: VoicingLayers,
        source_refresh: VoicingSourceRefresh,
        policies: VoicingPolicies,
        vvp: VvpVoicings,
    ) -> Self {
        Self {
            cache,
            layers,
            source_refresh,
            policies,
            vvp,
        }
    }

    /// patch の mono/poly を決める。画面側（keyboard / grid sequencer）の
    /// `*VoicingLookup` はどちらもこれ 1 本を呼ぶ。
    pub(in crate::tui) fn resolve(&self, patch: &str) -> Option<PatchVoicing> {
        let plugin = self.policies.plugin_for_patch(patch);
        let policy = VoicingPolicy::for_plugin(&plugin);
        match policy {
            VoicingPolicy::Sources => self.layers.resolve(&self.cache, patch),
            VoicingPolicy::AssumePoly => Some(PatchVoicing::Poly),
            VoicingPolicy::VvpHeader => self.vvp.voicing(&plugin, patch),
        }
    }

    /// 一覧に載っている `.vvp` を先に全部読む。読んだ件数を返す。
    pub(in crate::tui) fn prefetch_vvp_voicings(&self, pairs: &[(String, String)]) -> usize {
        self.vvp.prefetch(&self.policies.plugins, pairs)
    }
}

fn fallback_plugins(cfg: &Config) -> PatchPlugins {
    let dirs = cmrt_runtime::configured_patch_dirs(cfg);
    PatchPlugins::from_catalog(vec![CatalogPlugin {
        name: cfg.active_plugin.clone().unwrap_or_default(),
        plugin_path: cfg.plugin_path.clone(),
        plugin_id: cfg.plugin_id.clone(),
        base: cmrt_runtime::shared_patch_root_dir(&dirs),
        dirs,
        resolved_patches: None,
        source_notices: Vec::new(),
        patch_roles: cmrt_runtime::PatchRoles::resolve_for_default_plugin(
            cfg,
            &cfg.active_patch_roles,
        ),
    }])
}

#[cfg(test)]
mod tests;
