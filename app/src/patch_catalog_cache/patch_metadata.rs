//! catalog の音色分類と mono/poly 情報を作る。

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use cmrt_runtime::CatalogPlugin;

pub(super) fn describe_patches(
    plugins: &[CatalogPlugin],
    pairs: &[(String, String)],
) -> Result<Vec<cmrt_core::AudioPatch>> {
    let patch_plugins = cmrt_tui_core::patch_plugins::PatchPlugins::from_catalog(plugins.to_vec());
    pairs
        .iter()
        .map(|(display, _)| {
            let index = patch_plugins
                .index_for_patch(display)
                .map_err(anyhow::Error::new)?;
            let info = patch_plugins
                .audio_info(index)
                .with_context(|| format!("plugin情報がありません: {display}"))?;
            Ok(info.describe_patch(display, None))
        })
        .collect()
}

pub(super) fn collect_patch_voicings(
    plugins: &[CatalogPlugin],
    patches: &[cmrt_core::AudioPatch],
) -> BTreeMap<String, cmrt_realtime_play::PatchVoicing> {
    let patch_plugins = cmrt_tui_core::patch_plugins::PatchPlugins::from_catalog(plugins.to_vec());
    patches
        .iter()
        .filter_map(|patch| {
            let info = patch_plugins.audio_info_for_ref(&patch.reference).ok()?;
            if info.voicing_source() != cmrt_core::PluginVoicingSource::CatalogMetadata {
                return None;
            }
            let voicing = match patch.voicing {
                cmrt_core::PatchVoicingHint::Known { voicing } => local_voicing(voicing),
                cmrt_core::PatchVoicingHint::ExternalLookup { .. } => return None,
            };
            Some((patch.reference.display.clone(), voicing))
        })
        .collect()
}

fn local_voicing(voicing: cmrt_core::AdapterPatchVoicing) -> cmrt_realtime_play::PatchVoicing {
    match voicing {
        cmrt_core::AdapterPatchVoicing::Mono => cmrt_realtime_play::PatchVoicing::Mono,
        cmrt_core::AdapterPatchVoicing::Poly => cmrt_realtime_play::PatchVoicing::Poly,
        cmrt_core::AdapterPatchVoicing::Unknown => cmrt_realtime_play::PatchVoicing::Unknown,
    }
}
