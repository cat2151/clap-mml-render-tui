//! アプリ設定（`cmrt_runtime::Config`）からレンダリング用の [`CoreConfig`] を組み立てる。
//!
//! `CoreConfig` は play server repo の core crate の型なので、構造体リテラルを組む処理は
//! それを包むこの crate に置く。config crate の `cmrt-runtime` に置くと、play server 側から
//! 見たときに TUI の core-lib まで引きずり込む。

use cmrt_runtime::{catalog_plugins, CatalogPlugin, Config};

use crate::CoreConfig;

/// 既定プラグイン（音色を無指定にした行が鳴るもの）ぶんの `CoreConfig`。
/// notepad / DAW / offline render / server が共有する。その音色を鳴らすプラグインぶんが
/// 要るときは [`core_config_for_plugin`]。
pub fn core_config_from_config(cfg: &Config) -> CoreConfig {
    core_config_for_plugin(cfg, &catalog_plugins(cfg)[0])
}

/// カタログ上の 1 プラグインぶんの `CoreConfig`。`plugin_id` と `patches_dir`（display 文字列の
/// 相対化 base）はプラグインごとに違い、別プラグインの base で MML 先頭 JSON のパスを解決すると
/// 存在しないファイルを掴むか、相対パスが絶対パス扱いになって音色が当たらない。
pub fn core_config_for_plugin(cfg: &Config, plugin: &CatalogPlugin) -> CoreConfig {
    CoreConfig {
        plugin_id: plugin.plugin_id.clone(),
        output_midi: cfg.output_midi.clone(),
        output_wav: cfg.output_wav.clone(),
        sample_rate: cfg.sample_rate,
        buffer_size: cfg.buffer_size,
        patch_path: None,
        patches_dir: plugin.base.clone(),
        random_patch: false,
    }
}

#[cfg(test)]
mod tests;
