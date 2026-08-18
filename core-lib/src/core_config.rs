//! アプリ設定（`cmrt_runtime::Config`）からレンダリング用の [`CoreConfig`] を組み立てる。
//!
//! `CoreConfig` は play server repo の core crate が持つ型なので、その構造体リテラルを
//! 組む処理はこの crate（play server core を包む側）に置く。config crate である
//! `cmrt-runtime` を play server 側から見たときに、TUI の core-lib まで引きずり込まない
//! ようにするための配置でもある。

use cmrt_runtime::{core_config_patch_root_dir, Config};

use crate::CoreConfig;

/// アプリ設定からレンダリング用の `CoreConfig` を組み立てる。
/// notepad / DAW / offline render / server の各経路で共有する。
pub fn core_config_from_config(cfg: &Config) -> CoreConfig {
    CoreConfig {
        output_midi: cfg.output_midi.clone(),
        output_wav: cfg.output_wav.clone(),
        sample_rate: cfg.sample_rate,
        buffer_size: cfg.buffer_size,
        patch_path: None,
        patches_dir: core_config_patch_root_dir(cfg),
        random_patch: false,
    }
}

#[cfg(test)]
mod tests;
