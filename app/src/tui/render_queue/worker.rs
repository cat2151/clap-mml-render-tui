use super::*;

pub(super) fn render_worker(inner: Arc<TuiRenderQueueInner>) {
    loop {
        let start = inner.pop_next_render();
        send_stale_skips(start.stale_waiters);
        let Some(work) = start.work else {
            continue;
        };
        let completion = render_work(&inner, &work);
        let waiters = inner.finish_render(&work.mml);
        send_completion(&work.mml, waiters, completion);
    }
}

fn render_work(inner: &TuiRenderQueueInner, work: &TuiRenderWork) -> TuiRenderCompletion {
    // SAFETY: entry_ptr は main() で生存が保証される PluginEntry へのポインタを usize 化して保持したもの。
    // ワーカーはその参照先を書き換えず読み取り専用で扱い、TuiApp の実行中のみ使われる。
    let entry_ref: &PluginEntry = unsafe { &*(inner.entry_ptr as *const PluginEntry) };
    let core_cfg = CoreConfig::from(inner.cfg.as_ref());
    let _active_render_guard =
        ActiveRenderGuard::new(Arc::clone(&inner.active_offline_render_count));
    let active_render_count = inner.active_offline_render_count.load(Ordering::Relaxed);
    let probe_context = match work.caller {
        TuiRenderCaller::Playback { session } => {
            log_notepad_event(format!(
                "play render start session={session} active={} mml=\"{}\"",
                active_render_count,
                truncate_for_log(&work.mml, 120)
            ));
            NativeRenderProbeContext::tui_playback(
                session,
                active_render_count,
                daw_cache_mml_hash(&work.mml),
                inner.cfg.offline_render_workers,
            )
        }
        TuiRenderCaller::Prefetch => {
            log_notepad_event(format!(
                "cache prefetch render start active={} mml=\"{}\"",
                active_render_count,
                truncate_for_log(&work.mml, 80)
            ));
            NativeRenderProbeContext::tui_prefetch(
                active_render_count,
                daw_cache_mml_hash(&work.mml),
                inner.cfg.offline_render_workers,
            )
        }
    };

    match mml_render_with_probe(&work.mml, &core_cfg, entry_ref, Some(&probe_context)) {
        Ok((samples, patch_name)) => TuiRenderCompletion::Rendered {
            samples,
            patch_name,
        },
        Err(error) => TuiRenderCompletion::RenderError(error.to_string()),
    }
}

fn send_stale_skips(stale_waiters: Vec<StaleTuiRenderWaiter>) {
    for stale in stale_waiters {
        if let TuiRenderWaiterKind::Playback { session, .. } = stale.waiter.kind {
            log_notepad_event(format!(
                "play render stale skip before-render session={session}"
            ));
        }
        let _ = stale.waiter.response_tx.send(TuiRenderResponse {
            mml: stale.mml,
            completion: TuiRenderCompletion::SkippedStalePlayback,
        });
    }
}

fn send_completion(mml: &str, waiters: Vec<TuiRenderWaiter>, completion: TuiRenderCompletion) {
    for waiter in waiters {
        let _ = waiter.response_tx.send(TuiRenderResponse {
            mml: mml.to_string(),
            completion: completion.clone(),
        });
    }
}
