use cmrt_tui_core::patch_plugins::{CatalogPlugin, PatchPlugins};

use crate::config::Config;
use crate::history::VoicingCache;
use crate::realtime_play::PatchVoicing;
use crate::voicing_sources::{VoicingLayers, VoicingSourceRefresh};

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
}

impl VoicingPolicy {
    fn for_plugin(plugin: &CatalogPlugin) -> Self {
        if plugin.is_surge_xt() {
            Self::Sources
        } else {
            Self::AssumePoly
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
}

impl VoicingPolicies {
    pub(in crate::tui) fn from_config(cfg: &Config) -> Self {
        Self {
            plugins: PatchPlugins::from_config(cfg),
        }
    }

    fn for_patch(&self, patch: &str) -> VoicingPolicy {
        VoicingPolicy::for_plugin(self.plugins.for_patch(patch))
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
}

impl VoicingState {
    pub(in crate::tui) fn new(
        cache: VoicingCache,
        layers: VoicingLayers,
        source_refresh: VoicingSourceRefresh,
        policies: VoicingPolicies,
    ) -> Self {
        Self {
            cache,
            layers,
            source_refresh,
            policies,
        }
    }

    /// patch の mono/poly を決める。画面側（keyboard / grid sequencer）の
    /// `*VoicingLookup` はどちらもこれ 1 本を呼ぶ。
    pub(in crate::tui) fn resolve(&self, patch: &str) -> Option<PatchVoicing> {
        match self.policies.for_patch(patch) {
            VoicingPolicy::Sources => self.layers.resolve(&self.cache, patch),
            VoicingPolicy::AssumePoly => Some(PatchVoicing::Poly),
        }
    }
}

#[cfg(test)]
mod tests;
