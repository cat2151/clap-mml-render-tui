#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeRenderCallerKind {
    CacheWorker,
    PlaybackCurrent,
    PlaybackLookahead,
    Preview,
    PreviewPrefetch,
    TuiPlayback,
    TuiPrefetch,
}

impl NativeRenderCallerKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::CacheWorker => "cache_worker",
            Self::PlaybackCurrent => "playback_current",
            Self::PlaybackLookahead => "playback_lookahead",
            Self::Preview => "preview",
            Self::PreviewPrefetch => "preview_prefetch",
            Self::TuiPlayback => "tui_playback",
            Self::TuiPrefetch => "tui_prefetch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NativeRenderProbeDetails {
    CacheWorker {
        track: usize,
        measure: usize,
        generation: u64,
        rendered_mml_hash: u64,
    },
    TrackRender {
        track: usize,
        measure_index: usize,
        active_track_count: usize,
        snapshot_hash: u64,
    },
    TuiRender {
        session: Option<u64>,
        active_render_count: usize,
        snapshot_hash: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NativeRenderSnapshotKey {
    CacheWorker {
        track: usize,
        measure: usize,
        generation: u64,
        rendered_mml_hash: u64,
    },
    TrackRender {
        track: usize,
        measure_index: usize,
        snapshot_hash: u64,
    },
    TuiRender {
        snapshot_hash: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRenderProbeContext {
    caller_kind: NativeRenderCallerKind,
    offline_render_workers: usize,
    details: NativeRenderProbeDetails,
}

impl NativeRenderProbeContext {
    pub fn cache_worker(
        track: usize,
        measure: usize,
        generation: u64,
        rendered_mml_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::CacheWorker,
            offline_render_workers,
            details: NativeRenderProbeDetails::CacheWorker {
                track,
                measure,
                generation,
                rendered_mml_hash,
            },
        }
    }

    pub fn playback_current(
        track: usize,
        measure_index: usize,
        active_track_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::PlaybackCurrent,
            offline_render_workers,
            details: NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                active_track_count,
                snapshot_hash,
            },
        }
    }

    pub fn playback_lookahead(
        track: usize,
        measure_index: usize,
        active_track_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::PlaybackLookahead,
            offline_render_workers,
            details: NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                active_track_count,
                snapshot_hash,
            },
        }
    }

    pub fn preview(
        track: usize,
        measure_index: usize,
        active_track_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::Preview,
            offline_render_workers,
            details: NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                active_track_count,
                snapshot_hash,
            },
        }
    }

    pub fn preview_prefetch(
        track: usize,
        measure_index: usize,
        active_track_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::PreviewPrefetch,
            offline_render_workers,
            details: NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                active_track_count,
                snapshot_hash,
            },
        }
    }

    pub fn tui_playback(
        session: u64,
        active_render_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::TuiPlayback,
            offline_render_workers,
            details: NativeRenderProbeDetails::TuiRender {
                session: Some(session),
                active_render_count,
                snapshot_hash,
            },
        }
    }

    pub fn tui_prefetch(
        active_render_count: usize,
        snapshot_hash: u64,
        offline_render_workers: usize,
    ) -> Self {
        Self {
            caller_kind: NativeRenderCallerKind::TuiPrefetch,
            offline_render_workers,
            details: NativeRenderProbeDetails::TuiRender {
                session: None,
                active_render_count,
                snapshot_hash,
            },
        }
    }

    fn snapshot_key(&self) -> NativeRenderSnapshotKey {
        match &self.details {
            NativeRenderProbeDetails::CacheWorker {
                track,
                measure,
                generation,
                rendered_mml_hash,
            } => NativeRenderSnapshotKey::CacheWorker {
                track: *track,
                measure: *measure,
                generation: *generation,
                rendered_mml_hash: *rendered_mml_hash,
            },
            NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                snapshot_hash,
                ..
            } => NativeRenderSnapshotKey::TrackRender {
                track: *track,
                measure_index: *measure_index,
                snapshot_hash: *snapshot_hash,
            },
            NativeRenderProbeDetails::TuiRender { snapshot_hash, .. } => {
                NativeRenderSnapshotKey::TuiRender {
                    snapshot_hash: *snapshot_hash,
                }
            }
        }
    }

    pub(super) fn format_fields(&self) -> String {
        match &self.details {
            NativeRenderProbeDetails::CacheWorker {
                track,
                measure,
                generation,
                rendered_mml_hash,
            } => format!(
                "caller={} workers={} track={} measure={} generation={} rendered_mml_hash={}",
                self.caller_kind.as_str(),
                self.offline_render_workers,
                track,
                measure,
                generation,
                format_u64_hex(*rendered_mml_hash),
            ),
            NativeRenderProbeDetails::TrackRender {
                track,
                measure_index,
                active_track_count,
                snapshot_hash,
            } => format!(
                "caller={} workers={} track={} measure_index={} meas={} active_tracks={} snapshot_hash={}",
                self.caller_kind.as_str(),
                self.offline_render_workers,
                track,
                measure_index,
                measure_index + 1,
                active_track_count,
                format_u64_hex(*snapshot_hash),
            ),
            NativeRenderProbeDetails::TuiRender {
                session,
                active_render_count,
                snapshot_hash,
            } => match session {
                Some(session) => format!(
                    "caller={} workers={} session={} active_renders={} snapshot_hash={}",
                    self.caller_kind.as_str(),
                    self.offline_render_workers,
                    session,
                    active_render_count,
                    format_u64_hex(*snapshot_hash),
                ),
                None => format!(
                    "caller={} workers={} active_renders={} snapshot_hash={}",
                    self.caller_kind.as_str(),
                    self.offline_render_workers,
                    active_render_count,
                    format_u64_hex(*snapshot_hash),
                ),
            },
        }
    }

    pub(super) fn caller_kind_as_str(&self) -> &'static str {
        self.caller_kind.as_str()
    }

    pub(super) fn has_same_caller_kind_as(&self, other: &Self) -> bool {
        self.caller_kind == other.caller_kind
    }

    pub(super) fn has_same_snapshot_as(&self, other: &Self) -> bool {
        self.snapshot_key() == other.snapshot_key()
    }
}

fn format_u64_hex(value: u64) -> String {
    format!("0x{value:016x}")
}
