use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use cmrt_runtime::RealtimeAudioBackend;

use super::cache::{mark_cache_entry_recent, resolve_cached_samples, try_insert_cache};
use super::render_queue::TuiRenderCompletion;
use super::{disk_cache, truncate_for_log, PlayState};
use crate::NotepadScreen;

fn smf_playback_duration(
    smf_bytes: &[u8],
    sample_rate: f64,
) -> anyhow::Result<std::time::Duration> {
    let (_, total_samples) = cmrt_core::midi::parse_smf_bytes(smf_bytes, sample_rate)?;
    Ok(std::time::Duration::from_secs_f64(
        total_samples as f64 / sample_rate,
    ))
}

fn sleep_for_playback_session(
    playback_session: &AtomicU64,
    session: u64,
    duration: std::time::Duration,
) {
    let deadline = std::time::Instant::now() + duration;
    while playback_session.load(Ordering::Acquire) == session {
        let now = std::time::Instant::now();
        if now >= deadline {
            return;
        }
        std::thread::sleep((deadline - now).min(std::time::Duration::from_millis(10)));
    }
}

impl<'a> NotepadScreen<'a> {
    /// `mml` が `known_disk_cache_hashes` に含まれる場合のみ、ディスクキャッシュから
    /// サンプルを読み込んで返す。含まれない場合はディスクへ一切アクセスしない。
    pub(crate) fn resolve_disk_fallback_samples(
        &self,
        mml: &str,
        sample_rate: u32,
    ) -> Option<Vec<f32>> {
        let hash = cmrt_history::daw_cache_mml_hash(mml);
        if !self.audio.known_disk_hashes.lock().unwrap().contains(&hash) {
            return None;
        }
        disk_cache::load_valid_cached_wav(mml, sample_rate)
    }

    pub(crate) fn kick_play(&self, mml: String) {
        if self.cfg.realtime_audio_backend == RealtimeAudioBackend::PlayServer {
            self.kick_play_via_play_server(mml);
            return;
        }

        let cfg = Arc::clone(&self.cfg);
        let state = Arc::clone(self.playback.session.play_state());
        let playback_session = Arc::clone(self.playback.session.session_token());
        let active_sink = Arc::clone(self.playback.session.active_sink());
        let cache = Arc::clone(&self.audio.cache);
        let cache_order = Arc::clone(&self.audio.order);
        let render_queue = self.playback.render_queue.clone();
        let session = self.begin_playback_session();
        let mml_log = truncate_for_log(&mml, 120);

        let cache_guard = cache.lock().unwrap();
        let mut cached_samples = resolve_cached_samples(Some(&cache_guard), &mml);
        if cached_samples.is_some() {
            let mut cache_order = cache_order.lock().unwrap();
            mark_cache_entry_recent(&cache_guard, &mut cache_order, &mml);
        }
        drop(cache_guard);

        // オンメモリキャッシュがミスした場合でも、起動時またはflush時に走査した
        // ディスクキャッシュのハッシュ集合に含まれる行だけはディスクへフォールバックする。
        // 未知の行（新規に書いた行など）はこの集合に含まれないため、ここで無駄な
        // ディスクI/Oは発生しない。
        let mut cache_source = "hit";
        if cached_samples.is_none() {
            if let Some(samples) = self.resolve_disk_fallback_samples(&mml, cfg.sample_rate as u32)
            {
                let mut cache_guard = cache.lock().unwrap();
                let mut cache_order_guard = cache_order.lock().unwrap();
                try_insert_cache(
                    &mut cache_guard,
                    &mut cache_order_guard,
                    mml.clone(),
                    samples.clone(),
                    false,
                );
                cached_samples = Some(samples);
                cache_source = "disk-hit";
            }
        }

        if let Some(samples) = cached_samples {
            let msg = format!("(cached) | {}", mml);
            Self::log_notepad_event(format!(
                "play request session={session} cache={cache_source} mml=\"{mml_log}\""
            ));
            self.set_play_state_if_current(session, PlayState::Playing(msg.clone()));

            std::thread::spawn(move || {
                cmrt_tui_core::playback_session::play_samples_for_session(
                    &state,
                    &playback_session,
                    &active_sink,
                    session,
                    cfg.sample_rate as u32,
                    samples,
                    msg,
                );
            });
        } else {
            Self::log_notepad_event(format!(
                "play request session={session} cache=miss mml=\"{mml_log}\""
            ));
            self.set_play_state_if_current(session, PlayState::Running(mml.clone()));

            let response_rx = match render_queue.submit_playback(
                mml.clone(),
                session,
                Arc::clone(&playback_session),
            ) {
                Ok(response_rx) => response_rx,
                Err(error) => {
                    Self::log_notepad_event(format!(
                        "play render queue error session={session} err=\"{}\"",
                        truncate_for_log(&error.to_string(), 160)
                    ));
                    self.set_play_state_if_current(
                        session,
                        PlayState::Err(format!("エラー: {}", error)),
                    );
                    return;
                }
            };

            std::thread::spawn(move || {
                let response = match response_rx.recv() {
                    Ok(response) => response,
                    Err(_) => {
                        Self::log_notepad_event(format!(
                            "play render response dropped session={session}"
                        ));
                        cmrt_tui_core::playback_session::set_play_state_for_session(
                            &state,
                            &playback_session,
                            session,
                            PlayState::Err("エラー: render queue response dropped".to_string()),
                        );
                        return;
                    }
                };

                match response.completion {
                    TuiRenderCompletion::SkippedStalePlayback => {}
                    TuiRenderCompletion::RenderError(error) => {
                        Self::log_notepad_event(format!(
                            "play render error session={session} err=\"{}\"",
                            truncate_for_log(&error, 160)
                        ));
                        cmrt_tui_core::playback_session::set_play_state_for_session(
                            &state,
                            &playback_session,
                            session,
                            PlayState::Err(format!("エラー: {}", error)),
                        );
                    }
                    TuiRenderCompletion::Rendered {
                        samples,
                        patch_name,
                    } => {
                        if !cmrt_tui_core::playback_session::session_is_current(
                            &playback_session,
                            session,
                        ) {
                            Self::log_notepad_event(format!(
                                "play render stale skip after-render session={session}"
                            ));
                            return;
                        }
                        {
                            let mut cache = cache.lock().unwrap();
                            let mut cache_order = cache_order.lock().unwrap();
                            try_insert_cache(
                                &mut cache,
                                &mut cache_order,
                                mml.clone(),
                                samples.clone(),
                                false,
                            );
                        }

                        let msg = format!("{} | {}", patch_name, mml);
                        Self::log_notepad_event(format!(
                            "play render ok session={session} patch=\"{}\"",
                            truncate_for_log(&patch_name, 120)
                        ));
                        cmrt_tui_core::playback_session::set_play_state_for_session(
                            &state,
                            &playback_session,
                            session,
                            PlayState::Playing(msg.clone()),
                        );
                        cmrt_tui_core::playback_session::play_samples_for_session(
                            &state,
                            &playback_session,
                            &active_sink,
                            session,
                            cfg.sample_rate as u32,
                            samples,
                            msg,
                        );
                    }
                }
            });
        }
    }

    fn kick_play_via_play_server(&self, mml: String) {
        let Some(play_server) = self.playback.session.realtime_play_server().cloned() else {
            let session = self.begin_playback_session();
            self.set_play_state_if_current(
                session,
                PlayState::Err("エラー: realtime play server backend is not initialized".into()),
            );
            return;
        };

        let cfg = Arc::clone(&self.cfg);
        let state = Arc::clone(self.playback.session.play_state());
        let playback_session = Arc::clone(self.playback.session.session_token());
        let session = self.begin_playback_session();
        let mml_log = truncate_for_log(&mml, 120);
        let msg = format!("play-server | {}", mml);
        Self::log_notepad_event(format!(
            "play-server request session={session} mml=\"{mml_log}\""
        ));
        self.set_play_state_if_current(session, PlayState::Running(mml.clone()));

        std::thread::spawn(move || {
            // 行頭 JSON のパッチ指定を解決済みパスへ正規化して /play-mml へ送る。
            // SMF は再生時間の計算と旧サーバーへのフォールバック用。
            let core_cfg = cmrt_core::core_config_from_config(cfg.as_ref());
            let resolved_mml = cmrt_core::mml_with_resolved_embedded_patch(&mml, &core_cfg);
            let smf_bytes = match cmrt_core::mml_to_smf_bytes(resolved_mml.as_ref()) {
                Ok(smf_bytes) => smf_bytes,
                Err(error) => {
                    cmrt_tui_core::playback_session::set_play_state_for_session(
                        &state,
                        &playback_session,
                        session,
                        PlayState::Err(format!("エラー: SMF conversion failed: {error}")),
                    );
                    return;
                }
            };
            let duration = match smf_playback_duration(&smf_bytes, cfg.sample_rate) {
                Ok(duration) => duration,
                Err(error) => {
                    cmrt_tui_core::playback_session::set_play_state_for_session(
                        &state,
                        &playback_session,
                        session,
                        PlayState::Err(format!("エラー: SMF parse failed: {error}")),
                    );
                    return;
                }
            };
            if !cmrt_tui_core::playback_session::session_is_current(&playback_session, session) {
                return;
            }
            match play_server.play_mml(resolved_mml.as_ref(), smf_bytes) {
                Ok(()) => {
                    cmrt_tui_core::playback_session::set_play_state_for_session(
                        &state,
                        &playback_session,
                        session,
                        PlayState::Playing(msg.clone()),
                    );
                    sleep_for_playback_session(&playback_session, session, duration);
                    cmrt_tui_core::playback_session::set_play_state_for_session(
                        &state,
                        &playback_session,
                        session,
                        PlayState::Done(msg),
                    );
                }
                Err(error) => {
                    cmrt_tui_core::playback_session::set_play_state_for_session(
                        &state,
                        &playback_session,
                        session,
                        PlayState::Err(format!("エラー: {}", error)),
                    );
                }
            }
        });
    }
}
