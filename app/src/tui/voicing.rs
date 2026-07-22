use crate::history::VoicingCache;
use crate::voicing_sources::{VoicingLayers, VoicingSourceRefresh};

/// keyboard モードの voicing（patch ごとの mono/poly 判定）解決に必要な状態一式。
///
/// 判定結果キャッシュ（`cache`）・解決レイヤ（`layers`）・バックグラウンド更新
/// ハンドル（`source_refresh`）をまとめて保持する。これらは keyboard の voicing 解決で
/// 常に一緒に使われるため、所有を1型へ集約する。
pub(in crate::tui) struct VoicingState {
    pub(in crate::tui) cache: VoicingCache,
    pub(in crate::tui) layers: VoicingLayers,
    pub(in crate::tui) source_refresh: VoicingSourceRefresh,
}

impl VoicingState {
    pub(in crate::tui) fn new(
        cache: VoicingCache,
        layers: VoicingLayers,
        source_refresh: VoicingSourceRefresh,
    ) -> Self {
        Self {
            cache,
            layers,
            source_refresh,
        }
    }
}
