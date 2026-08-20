//! keyboard用の共有voicing判定を取得し、設定ディレクトリへ永続化する。
//!
//! 取得とキャッシュの実務は [`crate::cached_source`] が持つ。ここは voicing 固有の
//! 「shared / override の2層をどう読み、どう重ねるか」だけを扱う。

use std::{
    path::Path,
    sync::{Arc, Condvar, Mutex},
};

use anyhow::Result;
use cmrt_tui_core::patch_plugins::PatchPlugins;

use crate::{
    cached_source::CachedSource, config::Config, history::VoicingCache, realtime_play::PatchVoicing,
};

#[derive(Clone, Debug)]
struct SourceSet {
    shared: CachedSource,
    override_: CachedSource,
}

impl SourceSet {
    fn from_config(cfg: &Config) -> Option<Self> {
        Self::from_catalog(cfg, &PatchPlugins::from_config(cfg))
    }

    /// カタログを外から渡す形。**カタログは開発機のインストール状況で変わる**ので、
    /// テストはこちらを通す。
    fn from_catalog(cfg: &Config, plugins: &PatchPlugins) -> Option<Self> {
        // shared / override の JSON はキーが Surge の patch 表示パスで、Surge 以外の
        // プラグインでは 1 件も当たらない。取りに行くだけ無駄なので読まない
        // （Surge 以外の判定は `VoicingPolicy` が受け持つ）。
        //
        // 見るのは既定プラグインではなく**カタログ全体**。カタログに Surge の音色が
        // 1 つでも載るなら、既定プラグインが別でもこの JSON は要る。
        if !plugins.any_surge_xt() {
            return None;
        }
        let config_dir = crate::config::config_app_dir()?;
        Some(Self::new(
            &config_dir,
            cfg.voicing_shared_source.clone(),
            cfg.voicing_override_source.clone(),
        ))
    }

    fn new(config_dir: &Path, shared_source: String, override_source: String) -> Self {
        Self {
            shared: CachedSource::new("shared", config_dir, shared_source, "voicing/shared.json"),
            override_: CachedSource::new(
                "override",
                config_dir,
                override_source,
                "voicing/override.json",
            ),
        }
    }

    fn specs_missing_persistent_copy(&self) -> bool {
        [&self.shared, &self.override_]
            .into_iter()
            .any(CachedSource::missing_cache)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct VoicingLayers {
    shared: VoicingCache,
    override_: VoicingCache,
}

impl VoicingLayers {
    pub(crate) fn resolve(&self, user: &VoicingCache, patch: &str) -> Option<PatchVoicing> {
        self.override_
            .get(patch)
            .or_else(|| user.get(patch))
            .or_else(|| self.shared.get(patch))
    }
}

#[derive(Clone)]
pub(crate) struct VoicingSourceRefresh {
    sources: Option<SourceSet>,
    completion: Arc<(Mutex<bool>, Condvar)>,
    io_lock: Arc<Mutex<()>>,
}

impl VoicingSourceRefresh {
    pub(crate) fn spawn(cfg: &Config) -> Self {
        let refresh = Self {
            sources: SourceSet::from_config(cfg),
            completion: Arc::new((Mutex::new(false), Condvar::new())),
            io_lock: Arc::new(Mutex::new(())),
        };
        refresh.spawn_worker();
        refresh
    }

    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self {
            sources: None,
            completion: Arc::new((Mutex::new(true), Condvar::new())),
            io_lock: Arc::new(Mutex::new(())),
        }
    }

    fn spawn_worker(&self) {
        let sources = self.sources.clone();
        let completion = Arc::clone(&self.completion);
        let io_lock = Arc::clone(&self.io_lock);
        std::thread::spawn(move || {
            if let Some(sources) = sources {
                refresh_source_set(&sources, &io_lock, || completion.1.notify_all());
            }
            let (completed, signal) = &*completion;
            *completed.lock().unwrap() = true;
            signal.notify_all();
        });
    }

    pub(crate) fn load_for_keyboard(&self) -> VoicingLayers {
        let Some(sources) = &self.sources else {
            return VoicingLayers::default();
        };
        if sources.specs_missing_persistent_copy() {
            // キャッシュがまだ無いときだけ、ダウンロード完了を待つ。
            let (completed, signal) = &*self.completion;
            let mut completed = completed.lock().unwrap();
            while sources.specs_missing_persistent_copy() && !*completed {
                completed = signal.wait(completed).unwrap();
            }
        }
        let _guard = self.io_lock.lock().unwrap();
        VoicingLayers {
            shared: load_persisted_cache(&sources.shared),
            override_: load_persisted_cache(&sources.override_),
        }
    }
}

fn refresh_source_set(sources: &SourceSet, io_lock: &Mutex<()>, mut notify_progress: impl FnMut()) {
    let mut specs = [&sources.shared, &sources.override_];
    // キャッシュがまだ無い source を先に取りに行く（待たされるのはそちらだけなので）。
    specs.sort_by_key(|spec| spec.data_path.exists());
    for spec in specs {
        if !spec.enabled() {
            continue;
        }
        if let Err(error) = spec.refresh(io_lock, validate_voicing_json) {
            log_event(format!(
                "event=refresh-failed source={} error={error:#}",
                spec.label
            ));
        }
        notify_progress();
    }
}

fn validate_voicing_json(bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes)?;
    VoicingCache::from_shared_json(text)?;
    Ok(())
}

fn load_persisted_cache(spec: &CachedSource) -> VoicingCache {
    if !spec.enabled() {
        return VoicingCache::default();
    }
    let result = spec
        .read_cached()
        .and_then(|text| VoicingCache::from_shared_json(&text));
    match result {
        Ok(cache) => cache,
        Err(error) => {
            log_event(format!(
                "event=load-failed source={} error={error:#}",
                spec.label
            ));
            VoicingCache::default()
        }
    }
}

fn log_event(message: String) {
    #[cfg(not(test))]
    crate::logging::append_global_log_line(format!("voicing-source: {message}"));
    #[cfg(test)]
    let _ = message;
}

#[cfg(test)]
mod tests;
