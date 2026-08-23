//! 設定表示やテストデータで使うplugin identity定数。
//!
//! identityから挙動を選ぶ処理はplay-server shared core/configへ集約する。
pub use cmrt_server_config::{
    plugin_file_stem, DEXED_PLUGIN_ID, FLOE_PLUGIN_ID, SFORZANDO_PLUGIN_ID, SURGE_XT_PLUGIN_ID,
    VAPORIZER2_PLUGIN_ID,
};
