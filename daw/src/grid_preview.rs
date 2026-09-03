//! Grid履歴をDaily DAWへ書き込む前に、同じMMLの1小節目だけを試聴する。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cmrt_core::NativeRenderProbeContext;

use crate::grid_import::{grid_song_snapshot, DawGridImportSong};
use crate::mml::{build_cell_mml_from_data, cell_has_content, measure_duration_samples_from_data};
use crate::preview::render::{
    insert_overlay_preview_cache, overlay_preview_cache_key, render_mixed_preview_tracks,
    MixedPreviewRenderRequest, PreviewRenderProgressPhase,
};
use crate::render_queue::{RenderPriority, RenderQueue};
use crate::FIRST_PLAYABLE_TRACK;

mod output;

use output::PreviewOutput;

/// Grid履歴previewの非同期状態。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum DawGridPreviewStatus {
    #[default]
    Idle,
    Rendering {
        completed: usize,
        total: usize,
    },
    Playing,
    Finished,
    Error(String),
}

/// render 済みの1小節と、そのとき測った mixer 初期値。
#[derive(Clone)]
struct RenderedPreview {
    samples: Arc<Vec<f32>>,
    /// 先頭1小節の track ごとの音量から決めた mixer 初期値（dB）。
    /// `samples` にはこの補正が既に掛かっているので、preview で聞こえるバランスが
    /// そのまま import 後の初期バランスになる。
    track_volumes_db: Vec<i32>,
}

#[derive(Clone)]
struct PreparedPreview {
    key: u64,
    measure_samples: usize,
    active_tracks: Vec<usize>,
    track_mmls: Vec<String>,
    track_gains: Vec<f32>,
}

#[derive(Clone)]
struct PreviewRenderRuntime {
    render_queue: RenderQueue,
    cache: Arc<Mutex<HashMap<u64, RenderedPreview>>>,
    in_flight: Arc<Mutex<Option<(u64, u64)>>>,
    pending: Arc<Mutex<Option<(PreparedPreview, RenderPriority, u64)>>>,
    output: PreviewOutput,
    render_generation: Arc<AtomicU64>,
    offline_render_workers: usize,
}

/// Grid画面の間だけ生存する、保存処理を持たないoffline preview player。
pub struct DawGridPreviewPlayer {
    cfg: Arc<cmrt_runtime::Config>,
    runtime: PreviewRenderRuntime,
}

impl DawGridPreviewPlayer {
    pub fn new(
        cfg: Arc<cmrt_runtime::Config>,
        plugin_entries: cmrt_offline_render::PluginEntries,
    ) -> Self {
        let render_queue = RenderQueue::new(
            Arc::clone(&cfg),
            plugin_entries,
            cfg.effective_offline_render_workers(),
        );
        Self::with_render_queue(cfg, render_queue)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn disabled_for_tests(cfg: Arc<cmrt_runtime::Config>) -> Self {
        Self::with_render_queue(cfg, RenderQueue::disabled_for_tests())
    }

    fn with_render_queue(cfg: Arc<cmrt_runtime::Config>, render_queue: RenderQueue) -> Self {
        let runtime = PreviewRenderRuntime {
            render_queue,
            cache: Arc::new(Mutex::new(HashMap::new())),
            in_flight: Arc::new(Mutex::new(None)),
            pending: Arc::new(Mutex::new(None)),
            output: PreviewOutput::new(cfg.sample_rate as u32),
            render_generation: Arc::new(AtomicU64::new(0)),
            offline_render_workers: cfg.effective_offline_render_workers(),
        };
        Self { cfg, runtime }
    }

    /// 選択中の1小節目を再生する。cache済みならoffline renderを繰り返さない。
    pub fn play(&self, song: DawGridImportSong) {
        let render_generation = self
            .runtime
            .render_generation
            .fetch_add(1, Ordering::AcqRel)
            + 1;
        self.runtime.pending.lock().unwrap().take();
        let output_generation = self.runtime.output.begin_preparing(song.tracks.len());
        let runtime = self.runtime.clone();
        let sample_rate = self.cfg.sample_rate;
        std::thread::spawn(move || {
            let prepared = match prepare_first_measure(song, sample_rate) {
                Ok(prepared) => prepared,
                Err(error) => {
                    runtime
                        .output
                        .fail_to_prepare(output_generation, error.to_string());
                    return;
                }
            };
            if runtime.render_generation.load(Ordering::Acquire) != render_generation
                || !runtime.output.finish_preparing(
                    output_generation,
                    prepared.key,
                    prepared.active_tracks.len(),
                )
            {
                return;
            }
            ensure_render(runtime, prepared, RenderPriority::High, render_generation);
        });
    }

    pub fn stop(&self) {
        self.runtime
            .render_generation
            .fetch_add(1, Ordering::AcqRel);
        self.runtime.pending.lock().unwrap().take();
        self.runtime.output.stop();
    }

    pub fn status(&self) -> DawGridPreviewStatus {
        self.runtime.output.status()
    }

    /// preview 済みの曲なら、そのとき測った mixer 初期値を返す。
    ///
    /// preview を聴かずに import した場合や、まだ render が終わっていない場合は `None`。
    /// 呼び出し側は DAW 側で meas1 の cache が揃ってから決め直すこと。
    pub fn track_volumes_db(&self, song: &DawGridImportSong) -> Option<Vec<i32>> {
        let prepared = prepare_first_measure(song.clone(), self.cfg.sample_rate).ok()?;
        let rendered = self
            .runtime
            .cache
            .lock()
            .unwrap()
            .get(&prepared.key)
            .cloned()?;
        Some(rendered.track_volumes_db)
    }

    #[cfg(test)]
    fn ensure_render(&self, prepared: PreparedPreview, priority: RenderPriority) {
        let render_generation = self.runtime.render_generation.load(Ordering::Acquire);
        ensure_render(self.runtime.clone(), prepared, priority, render_generation);
    }
}

/// 同時にrenderする履歴は1件だけに制限し、連続移動では最後の選択だけを待機させる。
/// 1履歴のtrack群は`render_mixed_preview_tracks`内で並列になる。
fn ensure_render(
    runtime: PreviewRenderRuntime,
    prepared: PreparedPreview,
    priority: RenderPriority,
    render_generation: u64,
) {
    if runtime.render_generation.load(Ordering::Acquire) != render_generation {
        return;
    }
    if let Some(rendered) = runtime.cache.lock().unwrap().get(&prepared.key).cloned() {
        let mut pending = runtime.pending.lock().unwrap();
        if runtime.render_generation.load(Ordering::Acquire) != render_generation {
            return;
        }
        pending.take();
        drop(pending);
        runtime
            .output
            .start_if_desired(prepared.key, rendered.samples);
        return;
    }
    {
        let mut in_flight = runtime.in_flight.lock().unwrap();
        if *in_flight == Some((prepared.key, render_generation)) {
            let mut pending = runtime.pending.lock().unwrap();
            if runtime.render_generation.load(Ordering::Acquire) == render_generation {
                pending.take();
            }
            return;
        }
        if in_flight.is_some() {
            let mut pending = runtime.pending.lock().unwrap();
            if runtime.render_generation.load(Ordering::Acquire) == render_generation {
                *pending = Some((prepared, priority, render_generation));
            }
            return;
        }
        if runtime.render_generation.load(Ordering::Acquire) != render_generation {
            return;
        }
        *in_flight = Some((prepared.key, render_generation));
    }

    start_render_worker(runtime, prepared, priority, render_generation);
}

fn start_render_worker(
    runtime: PreviewRenderRuntime,
    prepared: PreparedPreview,
    priority: RenderPriority,
    render_generation: u64,
) {
    std::thread::spawn(move || {
        let key = prepared.key;
        if runtime.render_generation.load(Ordering::Acquire) != render_generation {
            finish_render_slot(runtime);
            return;
        }
        let total = prepared.active_tracks.len();
        let render = render_mixed_preview_tracks(
            &runtime.render_queue,
            MixedPreviewRenderRequest {
                priority,
                measure_samples: prepared.measure_samples,
                active_tracks: &prepared.active_tracks,
                track_mmls: &prepared.track_mmls,
                track_gains: &prepared.track_gains,
                // Grid history は gain 1.0 の素の音量で render するので、ここで測った
                // 音量差がそのまま mixer 初期値になる。
                auto_trim: true,
            },
            |track, mml| {
                NativeRenderProbeContext::preview(
                    track,
                    0,
                    total,
                    cmrt_history::daw_cache_mml_hash(mml),
                    runtime.offline_render_workers,
                )
            },
            |progress| {
                let completed = match progress.phase {
                    PreviewRenderProgressPhase::Started => progress.completed,
                    PreviewRenderProgressPhase::Done { .. }
                    | PreviewRenderProgressPhase::Error { .. } => progress.completed,
                };
                runtime.output.set_render_progress(key, completed, total);
            },
        );

        if let Some(render) = render {
            let sample_count = render.samples.len();
            let rendered = RenderedPreview {
                samples: Arc::new(render.samples),
                track_volumes_db: render
                    .auto_trim_volumes_db
                    .unwrap_or_else(|| vec![0; prepared.track_mmls.len()]),
            };
            insert_overlay_preview_cache(
                &mut runtime.cache.lock().unwrap(),
                key,
                sample_count,
                rendered.clone(),
            );
            runtime.output.start_if_desired(key, rendered.samples);
        } else {
            runtime
                .output
                .set_error_if_desired(key, "offline renderに失敗しました");
        }

        finish_render_slot(runtime);
    });
}

/// 完了したrender枠を解放し、待機中の最新候補があれば同じロック区間で次の枠を渡す。
/// これにより停止直後の再openや、完了と同時の選択移動でも候補が取り残されない。
fn finish_render_slot(runtime: PreviewRenderRuntime) {
    let next = claim_pending_render(&runtime);
    if let Some((next, next_priority, next_generation)) = next {
        let cached = runtime.cache.lock().unwrap().get(&next.key).cloned();
        if let Some(rendered) = cached {
            runtime.output.start_if_desired(next.key, rendered.samples);
            finish_render_slot(runtime);
        } else {
            start_render_worker(runtime, next, next_priority, next_generation);
        }
    }
}

fn claim_pending_render(
    runtime: &PreviewRenderRuntime,
) -> Option<(PreparedPreview, RenderPriority, u64)> {
    let mut in_flight = runtime.in_flight.lock().unwrap();
    let next = runtime.pending.lock().unwrap().take();
    *in_flight = next
        .as_ref()
        .map(|(prepared, _, generation)| (prepared.key, *generation));
    next
}

impl Drop for DawGridPreviewPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn prepare_first_measure(
    song: DawGridImportSong,
    sample_rate: f64,
) -> anyhow::Result<PreparedPreview> {
    let snapshot = grid_song_snapshot(song)?;
    let track_mmls = (0..snapshot.tracks)
        .map(|track| {
            if track >= FIRST_PLAYABLE_TRACK && cell_has_content(&snapshot.data, track, 1) {
                build_cell_mml_from_data(&snapshot.data, snapshot.measures, track, 1)
            } else {
                String::new()
            }
        })
        .collect::<Vec<_>>();
    let active_tracks = (FIRST_PLAYABLE_TRACK..snapshot.tracks)
        .filter(|track| grid_measure_has_attack(&track_mmls[*track]))
        .collect::<Vec<_>>();
    if active_tracks.is_empty() {
        anyhow::bail!("Grid historyの1小節目に音符がありません");
    }
    let track_gains = vec![1.0; snapshot.tracks];
    let measure_samples =
        measure_duration_samples_from_data(&snapshot.data, snapshot.measures, sample_rate);
    let key = overlay_preview_cache_key(0, &track_mmls, &track_gains);
    Ok(PreparedPreview {
        key,
        measure_samples,
        active_tracks,
        track_mmls,
        track_gains,
    })
}

/// Grid生成MMLの発音は必ずoctave指定`o`を持つ。restだけのtrackは、重いpatch
/// ロードとoffline renderをせずpreview対象から外す。
fn grid_measure_has_attack(mml: &str) -> bool {
    mmlabc_to_smf::mml_preprocessor::extract_embedded_json(mml)
        .remaining_mml
        .contains('o')
}

#[cfg(test)]
mod tests;
