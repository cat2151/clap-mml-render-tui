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

/// DAW モードのプレビューキャッシュ構築専用の MML → レンダリング。
/// - `patch_history.txt` への追記は行わない
/// - DAW 専用の MIDI/WAV キャッシュファイル（`daw_cache.*`）は生成しない
/// - ランダムパッチ選択は無効化し、通常レンダリングと同じ JSON 埋め込みパッチ解決を行う
///
/// # Parameters
/// - `mml`: レンダリング対象の MML 文字列。先頭 JSON によるパッチ指定も受け付ける。
/// - `cfg`: レンダリング設定。JSON にパッチ指定がない場合は `cfg.patch_path` を使う。
/// - `entry`: 読み込み済み CLAP プラグインエントリ。
///
/// # Returns
/// インターリーブされたステレオ PCM サンプル列を `Vec<f32>` で返す。
pub fn mml_render_for_cache(mml: &str, cfg: &CoreConfig, entry: &PluginEntry) -> Result<Vec<f32>> {
    let (patched_cfg, events, total_samples) = prepare_cache_render(mml, cfg)?;
    render::render_to_memory(&patched_cfg, entry, events, total_samples)
}

/// キャッシュレンダリング前処理を行い、メモリレンダリングに必要な入力を組み立てる。
///
/// 戻り値は `(patched_cfg, events, total_samples)` で、
/// それぞれ「JSON/既定パッチ適用後の設定」「SMF から展開した MIDI イベント列」
/// 「レンダリングすべき総サンプル数」を表す。
fn prepare_cache_render(
    mml: &str,
    cfg: &CoreConfig,
) -> Result<(CoreConfig, Vec<midi::TimedMidiEvent>, u64)> {
    let preprocessed = mml_preprocessor::extract_embedded_json(mml);
    // 通常レンダリングと同様に、先頭 JSON のパッチ指定を最優先し、
    // 指定がない場合だけ config の既定パッチにフォールバックする。
    let effective_patch = extract_patch_from_json(preprocessed.embedded_json.as_deref(), cfg)
        .or_else(|| cfg.patch_path.clone());

    let smf_bytes = mml_str_to_smf_bytes(&preprocessed.remaining_mml)?;
    let (events, total_samples) = midi::parse_smf_bytes(&smf_bytes, cfg.sample_rate)?;
    let patched_cfg = CoreConfig {
        patch_path: effective_patch,
        random_patch: false,
        ..cfg.clone()
    };

    Ok((patched_cfg, events, total_samples))
}

/// MML 先頭 JSON から `"Surge XT patch"` を取り出してパッチパスに解決する。
///
/// JSON に相対パスが入っていて `cfg.patches_dir` がある場合はその配下のパスに変換し、
/// `cfg.patches_dir` がない場合は JSON の文字列をそのまま返す。
fn extract_patch_from_json(json_str: Option<&str>, cfg: &CoreConfig) -> Option<String> {
    let json_str = json_str?;
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let rel = value.get("Surge XT patch")?.as_str()?;

    if let Some(ref base) = cfg.patches_dir {
        let abs = std::path::Path::new(base).join(std::path::Path::new(rel));
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
        let patches_dir = std::path::PathBuf::from("patches");
        let config = CoreConfig {
            output_midi: "out.mid".into(),
            output_wav: "out.wav".into(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patch_path: Some("/patches/Default.fxp".into()),
            patches_dir: Some(patches_dir.to_string_lossy().into_owned()),
            random_patch: true,
        };

        let patch =
            extract_patch_from_json(Some(r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#), &config);

        let expected = patches_dir.join("Pads").join("Pad 1.fxp");
        assert_eq!(patch.as_deref(), Some(expected.to_string_lossy().as_ref()));
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

    #[test]
    fn cache_render_prepares_memory_only_render_inputs() {
        let patches_dir = std::path::PathBuf::from("patches");
        let config = CoreConfig {
            output_midi: "out.mid".into(),
            output_wav: "out.wav".into(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patch_path: Some("patches/Default.fxp".into()),
            patches_dir: Some(patches_dir.to_string_lossy().into_owned()),
            random_patch: true,
        };

        let (patched_cfg, events, total_samples) = prepare_cache_render(
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"}t120o4c"#,
            &config,
        )
        .expect("cache render inputs should be prepared");

        assert_eq!(
            patched_cfg.patch_path.as_deref(),
            Some(
                patches_dir
                    .join("Pads")
                    .join("Pad 1.fxp")
                    .to_string_lossy()
                    .as_ref()
            )
        );
        assert!(
            !patched_cfg.random_patch,
            "random patch selection should be disabled for cache renders"
        );
        assert!(!events.is_empty(), "valid MML should produce MIDI events");
        assert!(total_samples > 0, "valid MML should produce a positive sample length");
    }
}
