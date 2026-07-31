//! grid sequencer の chord mode が使うコード進行カタログを取得し、設定ディレクトリへ
//! 永続化する。
//!
//! キャッシュがまだ無い初回だけ取得完了を待ち（待たないとカタログが空になり chord mode
//! を開始できない）、2回目以降はバックグラウンドで条件付き GET だけ行う。取得した内容が
//! 実際に変わっていたら [`ChordProgressionSource::take_update_notice`] が一度だけ true を
//! 返し、画面側が再起動アナウンスを出す。

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    time::Duration,
};

use anyhow::Result;
use cmrt_chord::ChordProgressionCatalog;

use crate::{
    cached_source::{CachedSource, SourceRefreshOutcome},
    config::Config,
};

/// 初回取得を待つ上限。ネットワークが死んでいるときに TUI の起動を止めないための保険。
const FIRST_LOAD_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone)]
pub(crate) struct ChordProgressionSource {
    source: Option<CachedSource>,
    completion: Arc<(Mutex<bool>, Condvar)>,
    io_lock: Arc<Mutex<()>>,
    updated: Arc<AtomicBool>,
}

impl ChordProgressionSource {
    pub(crate) fn spawn(cfg: &Config) -> Self {
        let source = crate::config::config_app_dir().map(|config_dir| {
            CachedSource::new(
                "chord-progressions",
                &config_dir,
                cfg.chord_progression_source.clone(),
                "chord-progressions/progressions.json",
            )
        });
        let refresh = Self {
            source,
            completion: Arc::new((Mutex::new(false), Condvar::new())),
            io_lock: Arc::new(Mutex::new(())),
            updated: Arc::new(AtomicBool::new(false)),
        };
        refresh.spawn_worker();
        refresh
    }

    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self {
            source: None,
            completion: Arc::new((Mutex::new(true), Condvar::new())),
            io_lock: Arc::new(Mutex::new(())),
            updated: Arc::new(AtomicBool::new(false)),
        }
    }

    fn spawn_worker(&self) {
        let source = self.source.clone();
        let completion = Arc::clone(&self.completion);
        let io_lock = Arc::clone(&self.io_lock);
        let updated = Arc::clone(&self.updated);
        std::thread::spawn(move || {
            if let Some(source) = source {
                if source.enabled() {
                    // キャッシュがまだ無い初回は、この結果が「更新」でも
                    // 再起動を促す意味がないので通知しない。
                    let had_cache = !source.missing_cache();
                    match source.refresh(&io_lock, validate_catalog_json) {
                        Ok(SourceRefreshOutcome::Updated) if had_cache => {
                            updated.store(true, Ordering::Release);
                            log_event("event=updated".to_string());
                        }
                        Ok(outcome) => log_event(format!("event=refreshed outcome={outcome:?}")),
                        Err(error) => log_event(format!("event=refresh-failed error={error:#}")),
                    }
                }
            }
            let (completed, signal) = &*completion;
            *completed.lock().unwrap() = true;
            signal.notify_all();
        });
    }

    /// コード進行カタログを読む。キャッシュがまだ無いときだけ取得完了を待つ。
    /// 取得も読み込みも失敗した場合は空のカタログを返す（chord mode は開始できない）。
    pub(crate) fn catalog(&self) -> ChordProgressionCatalog {
        let Some(source) = &self.source else {
            return ChordProgressionCatalog::default();
        };
        if !source.enabled() {
            return ChordProgressionCatalog::default();
        }
        if source.missing_cache() {
            let (completed, signal) = &*self.completion;
            let mut completed = completed.lock().unwrap();
            while source.missing_cache() && !*completed {
                let (guard, timeout) = signal
                    .wait_timeout(completed, FIRST_LOAD_TIMEOUT)
                    .expect("chord progression completion mutex is never poisoned");
                completed = guard;
                if timeout.timed_out() {
                    log_event("event=first-load-timeout".to_string());
                    break;
                }
            }
        }
        let _guard = self.io_lock.lock().unwrap();
        match source
            .read_cached()
            .and_then(|text| ChordProgressionCatalog::from_json(&text))
        {
            Ok(catalog) => catalog,
            Err(error) => {
                log_event(format!("event=load-failed error={error:#}"));
                ChordProgressionCatalog::default()
            }
        }
    }

    /// キャッシュ済みカタログが更新されたことを一度だけ報告する。
    pub(crate) fn take_update_notice(&self) -> bool {
        self.updated.swap(false, Ordering::AcqRel)
    }

    /// 取得スレッドの完了を待つ。実行時は待たない（待つのは初回取得だけ）ので、
    /// 取得結果を決め打ちで検証したいテスト専用。
    #[cfg(test)]
    fn wait_for_worker(&self) {
        let (completed, signal) = &*self.completion;
        let mut completed = completed.lock().unwrap();
        while !*completed {
            completed = signal.wait(completed).unwrap();
        }
    }
}

fn validate_catalog_json(bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes)?;
    ChordProgressionCatalog::from_json(text)?;
    Ok(())
}

fn log_event(message: String) {
    #[cfg(not(test))]
    crate::logging::append_global_log_line(format!("chord-progression-source: {message}"));
    #[cfg(test)]
    let _ = message;
}

#[cfg(test)]
mod tests;
