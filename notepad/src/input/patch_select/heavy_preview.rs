//! 重い音色（sample 総容量が大きい `.sfz`）の試聴。
//!
//! render が始まると止められず、その間 render worker を塞ぐ。cache に無ければカーソル移動では鳴らさず、
//! Space で確認を取ってから、鳴り始めるまで他の操作を受け付けずに待つ。cache にあれば軽い音色と同じく扱う。

use std::time::Instant;

use crossterm::event::KeyCode;

use crate::{NotepadScreen, PlayState};

pub(crate) enum HeavyPreview {
    /// 試聴してよいかを聞いている。
    Confirm {
        patch_name: String,
        sample_bytes: u64,
    },
    /// render を待っている。`mml` の render が終わるまで閉じない。
    Waiting {
        patch_name: String,
        mml: String,
        started: Instant,
    },
}

impl NotepadScreen<'_> {
    /// 選択中の音色が重く、試聴音が cache に無ければ、その sample 総容量。
    pub(crate) fn selected_heavy_sample_bytes(&self) -> Option<u64> {
        let select = self.patch_select.as_ref()?;
        let patch_name = select.selected()?;
        let measurement = select.load_measurement(patch_name)?;
        let bytes = measurement
            .is_heavy_offline_load()
            .then_some(measurement.sfz_sample_bytes)
            .flatten()?;
        let cached = self
            .patch_select_preview_mml_builder()
            .is_some_and(|preview_mml| {
                self.audio
                    .contains(&preview_mml.for_selector_patch(select, patch_name))
            });
        (!cached).then_some(bytes)
    }

    /// 自動試聴の代わりに、前の音色の音を止める。鳴り続けると今の音色の音と取り違える。
    pub(super) fn stop_preview_for_heavy_patch(&mut self) {
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
    }

    /// Space の試聴。重い音色なら確認を出す。
    pub(super) fn request_selected_patch_preview(&mut self) {
        let Some(sample_bytes) = self.selected_heavy_sample_bytes() else {
            self.preview_selected_patch();
            return;
        };
        let Some(patch_name) = self.patch_select_selected_patch_name() else {
            return;
        };
        self.heavy_preview = Some(HeavyPreview::Confirm {
            patch_name,
            sample_bytes,
        });
    }

    /// 確認・待機中のキー。待機中はどのキーも受け付けない。
    pub(super) fn handle_heavy_preview_key(&mut self, code: KeyCode) {
        let Some(HeavyPreview::Confirm { patch_name, .. }) = &self.heavy_preview else {
            return;
        };
        match code {
            KeyCode::Char('y') | KeyCode::Enter => {
                let patch_name = patch_name.clone();
                self.start_heavy_preview(patch_name);
            }
            KeyCode::Char('n') | KeyCode::Esc => self.heavy_preview = None,
            _ => {}
        }
    }

    fn start_heavy_preview(&mut self, patch_name: String) {
        let Some(mml) = self.patch_select.as_ref().and_then(|select| {
            self.patch_select_preview_mml_builder()
                .map(|preview_mml| preview_mml.for_selector_patch(select, &patch_name))
        }) else {
            self.heavy_preview = None;
            return;
        };
        Self::log_notepad_event(format!("tone-select heavy preview patch={patch_name:?}"));
        self.record_notepad_history(&mml);
        self.heavy_preview = Some(HeavyPreview::Waiting {
            patch_name,
            mml: mml.clone(),
            started: Instant::now(),
        });
        self.play_mml(mml);
    }

    /// render が終われば（鳴り始め・失敗のどちらでも）待機を閉じる。毎フレーム呼んでよい。
    pub fn pump_heavy_preview_wait(&mut self) {
        let Some(HeavyPreview::Waiting { mml, .. }) = &self.heavy_preview else {
            return;
        };
        let still_rendering = matches!(
            &*self.playback.session.play_state().lock().unwrap(),
            PlayState::Running(running) if running == mml
        );
        if !still_rendering {
            self.heavy_preview = None;
        }
    }
}
