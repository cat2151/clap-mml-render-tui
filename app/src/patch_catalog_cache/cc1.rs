//! Assess CC1-dependent pitch for every row in the catalog.

use std::{collections::BTreeMap, path::Path};

use anyhow::{ensure, Context, Result};
use cmrt_core::{load_entry, select_descriptor, AudioPluginInfo, PatchRef, PluginEntry};
use cmrt_tui_core::patch_catalog_cache::cc1::{
    Cache, Observation, Record, FILE_NAME, FORMAT_VERSION,
};
use cmrt_tui_core::patch_plugins::PatchPlugins;

use super::{cache_writer, report_progress, CacheFile};

mod audio;
mod pitch;
mod routing;
mod worker;

pub use worker::{run_cc1_worker, CC1_WORKER_COMMAND};

#[cfg(test)]
mod tests;

pub(super) fn prepare(
    catalog_path: &Path,
    catalog: &CacheFile,
) -> Result<impl FnOnce() -> Result<()>> {
    let path = catalog_path.with_file_name(FILE_NAME);
    let bytes = serde_json::to_vec_pretty(&collect(catalog)?)?;
    Ok(move || cache_writer::write_bytes(&path, &bytes))
}

fn collect(catalog: &CacheFile) -> Result<Cache> {
    let plugins =
        PatchPlugins::from_catalog(catalog.plugins.iter().cloned().map(Into::into).collect());
    let mut groups = BTreeMap::new();
    for row in &catalog.patches {
        let patch = &row.audio.reference;
        groups
            .entry(&patch.plugin)
            .or_insert_with(Vec::new)
            .push(patch);
    }
    let total = catalog.patches.len();
    let mut records = Vec::with_capacity(total);
    for (key, mut patches) in groups {
        patches.sort_unstable_by(|a, b| a.display.cmp(&b.display));
        ensure!(
            !patches.windows(2).any(|pair| pair[0] == pair[1]),
            "CC1: catalog に重複した PatchRef があります: {key}"
        );
        let library = plugins
            .audio_info_for_ref(patches[0])
            .map_err(Into::into)
            .and_then(Library::open);
        match &library {
            Ok(library) => report_progress(format!(
                "CC1 {} {}: {}件の調査を開始します",
                library.info.name,
                library.version,
                patches.len()
            )),
            Err(error) => report_progress(format!("CC1 {key}: CLAPを準備できません: {error:#}")),
        }
        let mut inspector = library.as_ref().ok().map(|library| {
            routing::Inspector::new(&library.id, &library.version, library.info, &library.entry)
        });
        let mut audio = library
            .as_ref()
            .ok()
            .map(|library| worker::Client::new(library.info, &library.id, &library.version));
        for patch in patches {
            let index = records.len() + 1;
            let label = || format!("CC1 [{index}/{total}] {}", patch.display);
            let observation = match (&library, &mut inspector, &mut audio) {
                (Ok(library), Some(inspector), Some(audio)) => {
                    library.observe(inspector, audio, patch, label)
                }
                (Err(error), _, _) => Observation::MeasurementFailed {
                    routing_unresolved: "CLAP descriptor を取得できず接続を確認できません".into(),
                    plugin_version: None,
                    error: format!("音声比較用の CLAP を開けません: {error:#}"),
                },
                _ => unreachable!(),
            };
            records.push(Record::new(patch.clone(), observation));
            if index % 1000 == 0 || index == total {
                report_progress(format!("CC1 {index}/{total}件を調査しました"));
            }
        }
    }
    Ok(Cache {
        format_version: FORMAT_VERSION,
        records,
    })
}

struct Library<'a> {
    info: &'a AudioPluginInfo,
    entry: PluginEntry,
    id: String,
    version: String,
}

impl<'a> Library<'a> {
    fn open(info: &'a AudioPluginInfo) -> Result<Self> {
        let entry = load_entry(&info.plugin_path)?;
        let descriptor = select_descriptor(&entry, info.plugin_id.as_deref())?;
        Ok(Self {
            info,
            entry,
            id: descriptor.id,
            version: descriptor.version,
        })
    }

    fn observe(
        &self,
        inspector: &mut routing::Inspector,
        audio: &mut worker::Client,
        patch: &PatchRef,
        label: impl FnOnce() -> String,
    ) -> Observation {
        let wire = patch.wire();
        let parsed = cmrt_core::parse_patch_wire(&wire).context("音色のwireを解釈できません");
        let (wire, path) = match parsed {
            Ok(wire) => (wire, self.info.patch_base.resolve(wire.path)),
            Err(error) => return self.failed("接続確認用の音色を特定できません".into(), error),
        };
        let reason = match inspector.observe(&path) {
            Ok(observation) => return observation,
            Err(error) => format!("{error:#}"),
        };
        let label = label();
        report_progress(format!("{label}: CC1=0/127 を描画・解析します"));
        let started = std::time::Instant::now();
        match audio.observe(wire.with_path(&path), reason.clone()) {
            Ok(observation) => {
                report_progress(format!(
                    "{label}: 音声比較完了 ({:.2}秒)",
                    started.elapsed().as_secs_f64()
                ));
                observation
            }
            Err(error) => {
                report_progress(format!("{label}: 音声比較に失敗: {error:#}"));
                self.failed(reason, error)
            }
        }
    }

    fn failed(&self, routing_unresolved: String, error: anyhow::Error) -> Observation {
        Observation::MeasurementFailed {
            routing_unresolved,
            plugin_version: Some(self.version.clone()),
            error: format!("{error:#}"),
        }
    }
}
