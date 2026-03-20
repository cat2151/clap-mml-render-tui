pub use clap_mml_play_server_core::patch_list::{collect_patches, to_relative};
pub use clap_mml_play_server_core::pipeline;
pub use clap_mml_play_server_core::pipeline::{
    ensure_cmrt_dir, ensure_daw_dir, ensure_phrase_dir, mml_render, mml_str_to_smf_bytes,
    mml_to_play, mml_to_smf_bytes, play_samples, write_wav,
};
pub use clap_mml_play_server_core::{host, load_entry, midi, patch_list, render, CoreConfig};

use anyhow::Result;
use clack_host::prelude::PluginEntry;
use mmlabc_to_smf::mml_preprocessor;

/// キャッシュ構築専用の MML → レンダリング。
/// - `patch_history.txt` への追記は行わない
/// - `daw_cache.*` のような意味のない一時ファイルは生成しない
/// - ランダムパッチ選択は無効化し、通常レンダリングと同じ JSON 埋め込みパッチ解決を行う
pub fn mml_render_for_cache(mml: &str, cfg: &CoreConfig, entry: &PluginEntry) -> Result<Vec<f32>> {
    let preprocessed = mml_preprocessor::extract_embedded_json(mml);
    let effective_patch = extract_patch_from_json(preprocessed.embedded_json.as_deref(), cfg)
        .or_else(|| cfg.patch_path.clone());

    let smf_bytes = mml_str_to_smf_bytes(&preprocessed.remaining_mml)?;
    let (events, total_samples) = midi::parse_smf_bytes(&smf_bytes, cfg.sample_rate)?;
    let patched_cfg = CoreConfig {
        patch_path: effective_patch,
        random_patch: false,
        ..cfg.clone()
    };

    render::render_to_memory(&patched_cfg, entry, events, total_samples)
}

fn extract_patch_from_json(json_str: Option<&str>, cfg: &CoreConfig) -> Option<String> {
    let json_str = json_str?;
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let rel = value.get("Surge XT patch")?.as_str()?;

    if let Some(ref base) = cfg.patches_dir {
        let abs = std::path::Path::new(base).join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        Some(abs.to_string_lossy().into_owned())
    } else {
        Some(rel.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn reexports_core_config() {
        let config = CoreConfig {
            output_midi: "out.mid".into(),
            output_wav: "out.wav".into(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patch_path: Some("/patches/Pad 1.fxp".into()),
            patches_dir: Some("/patches".into()),
            random_patch: false,
        };

        assert_eq!(config.output_midi, "out.mid");
        assert_eq!(config.output_wav, "out.wav");
        assert_eq!(config.sample_rate, 44_100.0);
        assert_eq!(config.buffer_size, 512);
        assert_eq!(config.patch_path.as_deref(), Some("/patches/Pad 1.fxp"));
        assert_eq!(config.patches_dir.as_deref(), Some("/patches"));
        assert!(!config.random_patch);
    }

    #[test]
    fn reexports_patch_helpers() {
        assert_eq!(
            to_relative("/patches", Path::new("/patches/Pads/Pad 1.fxp")),
            "Pads/Pad 1.fxp"
        );
    }

    #[test]
    fn cache_render_extracts_patch_from_embedded_json() {
        let config = CoreConfig {
            output_midi: "out.mid".into(),
            output_wav: "out.wav".into(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patch_path: Some("/patches/Default.fxp".into()),
            patches_dir: Some("/patches".into()),
            random_patch: true,
        };

        let patch =
            extract_patch_from_json(Some(r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#), &config);

        assert_eq!(patch.as_deref(), Some("/patches/Pads/Pad 1.fxp"));
    }

    #[test]
    fn cache_render_returns_none_when_json_patch_is_missing() {
        let config = CoreConfig {
            output_midi: "out.mid".into(),
            output_wav: "out.wav".into(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patch_path: Some("/patches/Default.fxp".into()),
            patches_dir: Some("/patches".into()),
            random_patch: true,
        };

        let patch = extract_patch_from_json(Some(r#"{"tempo":120}"#), &config);

        assert_eq!(patch, None);
    }
}
