//! Memoized voicing values produced by server-side plugin adapters.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use cmrt_tui_core::patch_plugins::{CatalogPlugin, PatchPlugins};

use crate::realtime_play::PatchVoicing;

#[derive(Clone, Default)]
pub(in crate::tui) struct CatalogVoicings {
    memo: Arc<Mutex<HashMap<String, Option<PatchVoicing>>>>,
}

impl CatalogVoicings {
    pub(in crate::tui) fn load_persisted(
        &self,
        entries: impl IntoIterator<Item = (String, PatchVoicing)>,
    ) -> usize {
        let mut memo = self.memo.lock().unwrap();
        memo.clear();
        memo.extend(entries.into_iter().map(|(patch, voicing)| {
            let decided = match voicing {
                PatchVoicing::Mono | PatchVoicing::Poly => Some(voicing),
                PatchVoicing::Unknown => None,
            };
            (patch, decided)
        }));
        memo.len()
    }

    pub(in crate::tui) fn voicing(
        &self,
        plugin: &CatalogPlugin,
        patch: &str,
    ) -> Option<PatchVoicing> {
        if let Some(memoized) = self.memo.lock().unwrap().get(patch) {
            return *memoized;
        }
        let info = cmrt_core::AudioPluginInfo::new(
            plugin.name.clone(),
            plugin.plugin_path.clone(),
            plugin.plugin_id.clone(),
            plugin.base.clone(),
        );
        let path = patch_file_path(plugin, patch);
        let decided = match info.describe_patch(patch, Some(&path)).voicing {
            cmrt_core::PatchVoicingHint::Known { voicing } => match voicing {
                cmrt_core::AdapterPatchVoicing::Mono => Some(PatchVoicing::Mono),
                cmrt_core::AdapterPatchVoicing::Poly => Some(PatchVoicing::Poly),
                cmrt_core::AdapterPatchVoicing::Unknown => None,
            },
            cmrt_core::PatchVoicingHint::ExternalLookup { .. } => None,
        };
        self.memo.lock().unwrap().insert(patch.to_string(), decided);
        decided
    }

    pub(in crate::tui) fn prefetch(
        &self,
        plugins: &PatchPlugins,
        pairs: &[(String, String)],
    ) -> usize {
        let mut read = 0usize;
        for (display, _) in pairs {
            let Ok(index) = plugins.index_for_patch(display) else {
                continue;
            };
            let Some(info) = plugins.audio_info(index) else {
                continue;
            };
            if info.voicing_source() != cmrt_core::PluginVoicingSource::CatalogMetadata {
                continue;
            }
            let Ok(plugin) = plugins.for_patch(display) else {
                continue;
            };
            self.voicing(plugin, display);
            read += 1;
        }
        read
    }
}

fn patch_file_path(plugin: &CatalogPlugin, patch: &str) -> PathBuf {
    match plugin.base.as_deref() {
        Some(base) => Path::new(base).join(patch),
        None => PathBuf::from(patch),
    }
}
