use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::DawGridPreviewStatus;

#[derive(Clone, Copy)]
struct DesiredPlay {
    key: u64,
    generation: u64,
    started: bool,
}

#[derive(Clone)]
pub(super) struct PreviewOutput {
    desired: Arc<Mutex<Option<DesiredPlay>>>,
    status: Arc<Mutex<DawGridPreviewStatus>>,
    generation: Arc<AtomicU64>,
    sink: Arc<Mutex<Option<Arc<rodio::Player>>>>,
    transition_lock: Arc<Mutex<()>>,
    sample_rate: u32,
}

impl PreviewOutput {
    pub(super) fn new(sample_rate: u32) -> Self {
        Self {
            desired: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(DawGridPreviewStatus::Idle)),
            generation: Arc::new(AtomicU64::new(0)),
            sink: Arc::new(Mutex::new(None)),
            transition_lock: Arc::new(Mutex::new(())),
            sample_rate,
        }
    }

    pub(super) fn begin_preparing(&self, total: usize) -> u64 {
        let _transition = self.transition_lock.lock().unwrap();
        self.stop_output_locked();
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        *self.desired.lock().unwrap() = None;
        *self.status.lock().unwrap() = DawGridPreviewStatus::Rendering {
            completed: 0,
            total,
        };
        generation
    }

    pub(super) fn finish_preparing(&self, generation: u64, key: u64, total: usize) -> bool {
        let _transition = self.transition_lock.lock().unwrap();
        if self.generation.load(Ordering::Acquire) != generation {
            return false;
        }
        *self.desired.lock().unwrap() = Some(DesiredPlay {
            key,
            generation,
            started: false,
        });
        *self.status.lock().unwrap() = DawGridPreviewStatus::Rendering {
            completed: 0,
            total,
        };
        true
    }

    pub(super) fn fail_to_prepare(&self, generation: u64, error: String) {
        let _transition = self.transition_lock.lock().unwrap();
        if self.generation.load(Ordering::Acquire) != generation {
            return;
        }
        self.stop_output_locked();
        *self.desired.lock().unwrap() = None;
        *self.status.lock().unwrap() = DawGridPreviewStatus::Error(error);
    }

    pub(super) fn stop(&self) {
        let _transition = self.transition_lock.lock().unwrap();
        self.generation.fetch_add(1, Ordering::AcqRel);
        *self.desired.lock().unwrap() = None;
        self.stop_output_locked();
        *self.status.lock().unwrap() = DawGridPreviewStatus::Idle;
    }

    pub(super) fn status(&self) -> DawGridPreviewStatus {
        self.status.lock().unwrap().clone()
    }

    pub(super) fn set_render_progress(&self, key: u64, completed: usize, total: usize) {
        let desired = self.desired.lock().unwrap();
        if desired
            .as_ref()
            .is_some_and(|wanted| wanted.key == key && !wanted.started)
        {
            *self.status.lock().unwrap() = DawGridPreviewStatus::Rendering { completed, total };
        }
    }

    pub(super) fn set_error_if_desired(&self, key: u64, message: &str) {
        let desired = self.desired.lock().unwrap();
        if desired
            .as_ref()
            .is_some_and(|wanted| wanted.key == key && !wanted.started)
        {
            *self.status.lock().unwrap() = DawGridPreviewStatus::Error(message.to_string());
        }
    }

    pub(super) fn start_if_desired(&self, key: u64, samples: Arc<Vec<f32>>) {
        let play_generation = {
            let mut desired = self.desired.lock().unwrap();
            let Some(wanted) = desired.as_mut() else {
                return;
            };
            if wanted.key != key || wanted.started {
                return;
            }
            wanted.started = true;
            wanted.generation
        };
        let output = self.clone();
        std::thread::spawn(move || output.play_samples(play_generation, samples));
    }

    fn play_samples(&self, play_generation: u64, samples: Arc<Vec<f32>>) {
        let Some(rodio_sample_rate) = rodio::SampleRate::new(self.sample_rate) else {
            self.set_error_if_generation(play_generation, "sample rateが0です");
            return;
        };
        let Ok(device_sink) = cmrt_tui_core::audio_output::open_default_sink() else {
            self.set_error_if_generation(play_generation, "audio出力を開けません");
            return;
        };
        let player = Arc::new(rodio::Player::connect_new(device_sink.mixer()));
        let source = rodio::buffer::SamplesBuffer::new(
            cmrt_tui_core::playback_session::STEREO,
            rodio_sample_rate,
            samples.as_ref().clone(),
        );
        {
            let _transition = self.transition_lock.lock().unwrap();
            if self.generation.load(Ordering::Acquire) != play_generation {
                return;
            }
            *self.sink.lock().unwrap() = Some(Arc::clone(&player));
            *self.status.lock().unwrap() = DawGridPreviewStatus::Playing;
            player.append(source);
        }
        player.sleep_until_end();
        {
            let _transition = self.transition_lock.lock().unwrap();
            if self.generation.load(Ordering::Acquire) == play_generation {
                self.sink.lock().unwrap().take();
                *self.status.lock().unwrap() = DawGridPreviewStatus::Finished;
            }
        }
    }

    fn set_error_if_generation(&self, generation: u64, message: &str) {
        let desired = self.desired.lock().unwrap();
        if desired
            .as_ref()
            .is_some_and(|wanted| wanted.generation == generation)
        {
            *self.status.lock().unwrap() = DawGridPreviewStatus::Error(message.to_string());
        }
    }

    fn stop_output_locked(&self) {
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
    }
}
