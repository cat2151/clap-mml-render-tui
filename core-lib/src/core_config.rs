//! アプリ設定（`cmrt_runtime::Config`）からレンダリング用の [`CoreConfig`] を組み立てる。
//!
//! `CoreConfig` は play server repo の core crate が持つ型なので、その構造体リテラルを
//! 組む処理はこの crate（play server core を包む側）に置く。config crate である
//! `cmrt-runtime` を play server 側から見たときに、TUI の core-lib まで引きずり込まない
//! ようにするための配置でもある。

use cmrt_runtime::{catalog_plugins, CatalogPlugin, Config};

use crate::CoreConfig;

/// アプリ設定からレンダリング用の `CoreConfig` を組み立てる。
/// notepad / DAW / offline render / server の各経路で共有する。
///
/// **既定プラグイン（音色を無指定にした行が鳴るもの）ぶんの `CoreConfig`。**
/// 混在カタログで「その音色を鳴らすプラグイン」ぶんが要るときは
/// [`core_config_for_plugin`] を使うこと。
pub fn core_config_from_config(cfg: &Config) -> CoreConfig {
    core_config_for_plugin(cfg, &catalog_plugins(cfg)[0])
}

/// カタログ上の 1 プラグインぶんの `CoreConfig` を組み立てる。
///
/// `plugin_id` と `patches_dir`（display 文字列の相対化 base）は**プラグインごとに違う**。
/// 別プラグインの base で MML 先頭 JSON のパスを解決すると、存在しないファイルを掴むか、
/// 相対パスがそのまま絶対パス扱いになって音色が当たらない。
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
