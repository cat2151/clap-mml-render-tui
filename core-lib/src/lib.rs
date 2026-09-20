// DAW の演奏ループが、キャッシュ WAV を「どのスロットへ載せるか」まで指定した
// patch 文字列を組み立てるために使う（`cache_wav_patch_with_slot` / `SLOT_COUNT`）。
// 綴りの単一ソースは play server 側 `core-lib/src/cache_wav.rs` の module doc。
pub use clap_mml_play_server_core::cache_wav;
pub use clap_mml_play_server_core::patch_list::{collect_patches, to_relative};
pub use clap_mml_play_server_core::pipeline;
pub use clap_mml_play_server_core::pipeline::{
    embedded_patch_ref, ensure_cmrt_dir, ensure_daw_dir, ensure_phrase_dir, mml_str_to_smf_bytes,
    mml_to_smf_bytes, play_samples, write_wav,
};
pub use clap_mml_play_server_core::EffectPlugins;
pub use clap_mml_play_server_core::PatchVoicing as AdapterPatchVoicing;
pub use clap_mml_play_server_core::{
    builtin_effect_plugins, effect_chain_spec_from_embedded_json, embedded_json_has_effect_chain,
    AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset, EffectChainSpec, EffectStageSpec,
    EFFECT_CHAIN_JSON_KEY, EFFECT_STAGE_BYPASS_JSON_KEY,
};
pub use clap_mml_play_server_core::{midi, patch_list, CoreConfig};
pub use clap_mml_play_server_core::{
    patch_lookup_candidates, patch_sort_metadata, plugin_voicing_source, AudioPatch,
    AudioPluginCatalog, AudioPluginInfo, PatchRef, PatchSortMetadata, PatchVoicingHint, PluginKey,
    PluginVoicingSource, RouteError,
};
pub use clap_mml_play_server_core::{set_log_sink, LogSink};

use mmlabc_to_smf::mml_preprocessor;
use std::borrow::Cow;

const PATCH_DIR_PREFIXES: [&str; 2] = ["patches_factory", "patches_3rdparty"];

mod cache_dirs;
mod core_config;

pub use cache_dirs::{
    cache_plugin_namespace, ensure_daw_cache_dir, ensure_notepad_cache_dir,
    init_cache_plugin_namespace, migrate_legacy_caches, DEFAULT_CACHE_PLUGIN_NAMESPACE,
};
pub use core_config::{core_config_for_plugin, core_config_from_config};

/// MML 先頭 JSON の `"Surge XT patch"` を解決済みのパス
/// （`patches_factory` 等のプレフィックス補完込み）に書き換えた MML を返す。
/// play server など JSON 込み MML を受け取る外部プロセスへ渡す前の正規化に使う。
/// 他のキー（effect chain など）はそのまま残す。
pub fn mml_with_resolved_embedded_patch<'a>(mml: &'a str, cfg: &CoreConfig) -> Cow<'a, str> {
    let preprocessed = mml_preprocessor::extract_embedded_json(mml);
    let Some(resolved_patch) = extract_patch_from_json(preprocessed.embedded_json.as_deref(), cfg)
    else {
        return Cow::Borrowed(mml);
    };
    let Some(embedded_json) = preprocessed.embedded_json.as_deref() else {
        return Cow::Borrowed(mml);
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(embedded_json) else {
        return Cow::Borrowed(mml);
    };
    let Some(object) = value.as_object_mut() else {
        return Cow::Borrowed(mml);
    };
    object.insert(
        "Surge XT patch".to_string(),
        serde_json::Value::String(patch_value_for_core_embedded_json(&resolved_patch, cfg)),
    );
    match serde_json::to_string(&value) {
        Ok(json) => Cow::Owned(format!("{json}{}", preprocessed.remaining_mml)),
        Err(_) => Cow::Borrowed(mml),
    }
}

fn patch_value_for_core_embedded_json(resolved_patch: &str, cfg: &CoreConfig) -> String {
    let resolved_path = std::path::Path::new(resolved_patch);
    if let Some(base) = cfg.patches_dir.as_deref() {
        if let Ok(relative_path) = resolved_path.strip_prefix(std::path::Path::new(base)) {
            return relative_path.to_string_lossy().into_owned();
        }
    }
    resolved_patch.to_string()
}

/// MML 先頭 JSON から `"Surge XT patch"` を取り出してパッチパスに解決する。
///
/// JSON に相対パスが入っていて `cfg.patches_dir` がある場合はその配下のパスに変換し、
/// `cfg.patches_dir` がない場合は JSON の文字列をそのまま返す。
fn extract_patch_from_json(json_str: Option<&str>, cfg: &CoreConfig) -> Option<String> {
    let json_str = json_str?;
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let rel = value.get("Surge XT patch")?.as_str()?;
    let rel_path = normalize_patch_path(rel);

    if let Some(ref base) = cfg.patches_dir {
        let base = std::path::Path::new(base);
        let abs = resolve_patch_path_from_base(base, &rel_path);
        Some(abs.to_string_lossy().into_owned())
    } else {
        Some(rel_path.to_string_lossy().into_owned())
    }
}

fn normalize_patch_path(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(rel).components().collect()
}

fn resolve_patch_path_from_base(
    base: &std::path::Path,
    rel_path: &std::path::Path,
) -> std::path::PathBuf {
    let abs = base.join(rel_path);
    if abs.exists() {
        return abs;
    }

    if rel_path.components().next().is_none() {
        return abs;
    }
    if rel_path
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .is_some_and(|first| {
            PATCH_DIR_PREFIXES
                .iter()
                .any(|prefix| first.eq_ignore_ascii_case(prefix))
        })
    {
        return abs;
    }

    for prefix in PATCH_DIR_PREFIXES {
        let candidate = base.join(prefix).join(rel_path);
        if candidate.exists() {
            return candidate;
        }
    }

    abs
}

#[cfg(test)]
mod tests;
