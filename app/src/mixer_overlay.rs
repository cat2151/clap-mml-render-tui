//! mixer オーバーレイ関連の再エクスポート。
//!
//! 音量ドメイン（定数・dB 調整・ゲイン変換）と描画ウィジェットは画面横断で共有するため
//! `cmrt-tui-core` へ切り出した。従来の `crate::mixer_overlay::*` パスは再エクスポートで維持する。

pub(crate) use cmrt_tui_core::mixer::{
    adjust_volume_db, volume_db_to_gain, MIXER_MAX_DB, MIXER_MIN_DB, MIXER_STEP_DB,
};
pub(crate) use cmrt_tui_core::mixer_overlay::{draw_mixer_overlay, MixerOverlayTrack};
