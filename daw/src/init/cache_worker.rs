use std::sync::{Arc, Mutex};

use crate::{CacheJob, CacheState, CellCache, WorkspaceKind, MAX_CACHED_SAMPLES};

pub(super) fn reserve_cache_job_for_render(
    cache: &Arc<Mutex<Vec<Vec<CellCache>>>>,
    job: &CacheJob,
) -> bool {
    let mut cache = cache.lock().unwrap();
    let Some(cell) = cache
        .get_mut(job.track)
        .and_then(|row| row.get_mut(job.measure))
    else {
        return false;
    };
    if cell.state == CacheState::Empty || cell.generation != job.generation {
        return false;
    }
    cell.state = CacheState::Rendering;
    cell.rendered_mml_hash = None;
    true
}

pub(super) fn mark_cache_job_error(cache: &Arc<Mutex<Vec<Vec<CellCache>>>>, job: &CacheJob) {
    let mut cache = cache.lock().unwrap();
    let Some(cell) = cache
        .get_mut(job.track)
        .and_then(|row| row.get_mut(job.measure))
    else {
        return;
    };
    if cell.generation != job.generation {
        return;
    }
    cell.state = CacheState::Error;
    cell.samples = None;
    cell.rendered_measure_samples = None;
    cell.rendered_mml_hash = None;
}

pub(super) fn store_cache_job_samples(
    cache: &Arc<Mutex<Vec<Vec<CellCache>>>>,
    job: &CacheJob,
    daw_cfg: &cmrt_runtime::Config,
    workspace_kind: WorkspaceKind,
    samples: Vec<f32>,
) -> bool {
    let mut cache = cache.lock().unwrap();
    let Some(cell) = cache
        .get_mut(job.track)
        .and_then(|row| row.get_mut(job.measure))
    else {
        return false;
    };
    if cell.generation != job.generation {
        return false;
    }

    // 開発用: track/measure ごとに WAV ファイルを出力する。
    // measure 0 は音色/ヘッダセルであり演奏内容ではないためスキップ。
    let wav_ok = if job.measure > 0 {
        if let Ok(daw_dir) = crate::cache::ensure_workspace_cache_dir(workspace_kind) {
            let wav_path = daw_dir.join(format!("track{}_meas{}.wav", job.track, job.measure));
            cmrt_core::write_wav(&samples, daw_cfg.sample_rate as u32, &wav_path).is_ok()
        } else {
            false
        }
    } else {
        true
    };

    cell.state = if wav_ok {
        CacheState::Ready
    } else {
        CacheState::Error
    };
    cell.rendered_mml_hash = if wav_ok {
        Some(job.rendered_mml_hash)
    } else {
        None
    };
    // Ready かつサイズ上限以内のときのみサンプルをメモリに保持する。
    // 上限超過（低 BPM 等）や WAV 失敗時はサンプルを保持しない。
    if wav_ok && samples.len() <= MAX_CACHED_SAMPLES {
        cell.samples = Some(Arc::new(samples));
        cell.rendered_measure_samples = Some(job.measure_samples);
    } else {
        cell.samples = None;
        cell.rendered_measure_samples = None;
    }
    true
}
